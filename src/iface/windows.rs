use super::{EthAddr, IfaceIdx};
use crate::error::OsError;
use std::ffi::{CStr, OsString};
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::windows::ffi::OsStringExt;
use std::{mem, ptr, slice};
use windows_sys::Win32::Foundation;
use windows_sys::Win32::NetworkManagement::IpHelper;
use windows_sys::Win32::NetworkManagement::Ndis;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::core::PWSTR;

const unsafe fn utf16_len(p: PWSTR) -> usize {
    let mut len = 0;
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    len
}

fn if_name2luid(if_name: &str) -> Result<Ndis::NET_LUID_LH, OsError> {
    let if_name: Vec<u16> = if_name.encode_utf16().collect();
    unsafe {
        let mut luid = MaybeUninit::<Ndis::NET_LUID_LH>::uninit();
        match IpHelper::ConvertInterfaceNameToLuidW(if_name.as_ptr(), luid.as_mut_ptr()) {
            Foundation::NO_ERROR => Err(OsError::last()),
            _ => Ok(luid.assume_init()),
        }
    }
}

fn if_luid2idx(luid: &Ndis::NET_LUID_LH) -> Result<IfaceIdx, OsError> {
    let mut ifi = 0;
    unsafe {
        match IpHelper::ConvertInterfaceLuidToIndex(luid, &mut ifi) {
            Foundation::NO_ERROR => Err(OsError::last()),
            _ => Ok(IfaceIdx { ifi: ifi }),
        }
    }
}

fn if_idx2luid(ifi: u32) -> Result<Ndis::NET_LUID_LH, OsError> {
    let mut luid = MaybeUninit::<Ndis::NET_LUID_LH>::uninit();
    unsafe {
        match IpHelper::ConvertInterfaceIndexToLuid(ifi, luid.as_mut_ptr()) {
            Foundation::NO_ERROR => Err(OsError::last()),
            _ => Ok(luid.assume_init()),
        }
    }
}

fn if_luid2name(luid: &Ndis::NET_LUID_LH) -> Result<String, OsError> {
    let mut buf = [0; Ndis::IF_MAX_STRING_SIZE as usize + 1];
    unsafe {
        match IpHelper::ConvertInterfaceLuidToNameW(luid, buf.as_mut_ptr(), buf.len() - 1) {
            Foundation::NO_ERROR => Err(OsError::last()),
            _ => {
                let len = utf16_len(buf.as_mut_ptr());
                let buf = OsString::from_wide(&buf[..len]);
                Ok(buf.into_string().unwrap())
            }
        }
    }
}

impl IfaceIdx {
    pub fn new(if_name: &str) -> Result<Self, OsError> {
        let luid = if_name2luid(if_name)?;
        if_luid2idx(&luid)
    }

    pub fn name(&self) -> Result<String, OsError> {
        let luid = if_idx2luid(self.ifi)?;
        if_luid2name(&luid)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PseudoPhysicalRef<'a> {
    idx: IfaceIdx,
    addr: Option<&'a EthAddr>,
}

impl<'a> PseudoPhysicalRef<'a> {
    pub const fn iface_idx(&self) -> IfaceIdx {
        self.idx
    }

    pub const fn eth_addr(&self) -> Option<&EthAddr> {
        self.addr
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum IfaceAddrRef<'a> {
    V4(&'a Ipv4Addr, u8),
    V6(&'a Ipv6Addr, u8, IfaceIdx),
    Hw(PseudoPhysicalRef<'a>),
}

pub struct IfaceRef<'a>(
    &'a IpHelper::IP_ADAPTER_ADDRESSES_LH,
    Option<&'a IpHelper::IP_ADAPTER_UNICAST_ADDRESS_LH>,
);

impl<'a> IfaceRef<'a> {
    pub fn name(&self) -> String {
        unsafe {
            let len = utf16_len(self.0.FriendlyName);
            let utf16 = slice::from_raw_parts(self.0.FriendlyName, len);
            let utf16 = OsString::from_wide(utf16);
            utf16.to_string_lossy().into_owned()
        }
    }

    /// The low-level, system-internal adapter name (e.g., "{A1B2C3D4-E5F6-7890-1234-567890ABCDEF}") used by the registry and NDIS.
    pub fn adapter_name(&self) -> &'a str {
        let s = unsafe { CStr::from_ptr(self.0.AdapterName.cast()) };
        s.to_str().unwrap()
    }

    const fn eth_addr(&self) -> Option<&'a EthAddr> {
        if self.0.PhysicalAddressLength == 6 {
            unsafe {
                let bytes: &[u8; 6] = mem::transmute(self.0.PhysicalAddress.as_ptr());
                Some(mem::transmute(bytes))
            }
        } else {
            None
        }
    }

    pub const fn addr(&self) -> IfaceAddrRef<'a> {
        if let Some(unicast) = &self.1 {
            let sa = unsafe { &*unicast.Address.lpSockaddr };
            match sa.sa_family {
                WinSock::AF_INET => unsafe {
                    let sin = &*(unicast.Address.lpSockaddr as *mut WinSock::SOCKADDR_IN);
                    let len = unicast.OnLinkPrefixLength;
                    IfaceAddrRef::V4(mem::transmute(&sin.sin_addr), len)
                },
                WinSock::AF_INET6 => unsafe {
                    let sin6 = &*(unicast.Address.lpSockaddr as *mut WinSock::SOCKADDR_IN6);
                    let len = unicast.OnLinkPrefixLength;
                    let idx = IfaceIdx::from_raw(sin6.Anonymous.sin6_scope_id);
                    IfaceAddrRef::V6(mem::transmute(&sin6.sin6_addr), len, idx)
                },
                _ => unreachable!(),
            }
        } else {
            IfaceAddrRef::Hw(PseudoPhysicalRef {
                idx: IfaceIdx {
                    ifi: self.0.Ipv6IfIndex,
                },
                addr: self.eth_addr(),
            })
        }
    }
}

pub struct IfaceIter<'a>(
    Option<&'a IpHelper::IP_ADAPTER_ADDRESSES_LH>,
    Option<&'a IpHelper::IP_ADAPTER_UNICAST_ADDRESS_LH>,
);

impl<'a> Iterator for IfaceIter<'a> {
    type Item = IfaceRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.0, self.1) {
            (None, _) => None,
            (Some(addrs), None) => {
                if addrs.FirstUnicastAddress.is_null() {
                    if addrs.Next.is_null() {
                        self.0 = None;
                        //self.1 = None;
                    } else {
                        self.0 = Some(unsafe { &*addrs.Next });
                        //self.1 = None;
                    }
                } else {
                    self.0 = Some(addrs);
                    self.1 = Some(unsafe { &*addrs.FirstUnicastAddress })
                }
                Some(IfaceRef(addrs, None))
            }
            (Some(addrs), Some(unicast)) => {
                if unicast.Next.is_null() {
                    if addrs.Next.is_null() {
                        self.0 = None;
                    } else {
                        self.0 = Some(unsafe { &*addrs.Next });
                    }
                    self.1 = None;
                } else {
                    self.0 = Some(addrs);
                    self.1 = Some(unsafe { &*unicast.Next });
                }
                Some(IfaceRef(addrs, Some(unicast)))
            }
        }
    }
}

struct IpAdapterAddresses(*mut IpHelper::IP_ADAPTER_ADDRESSES_LH);

impl Drop for IpAdapterAddresses {
    fn drop(&mut self) {
        unsafe { libc::free(self.0.cast()) }
    }
}

impl IpAdapterAddresses {
    fn new() -> Result<Self, OsError> {
        let mut dw_size: u32 = 0;

        unsafe {
            match IpHelper::GetAdaptersAddresses(
                WinSock::AF_UNSPEC as u32,
                IpHelper::GAA_FLAG_INCLUDE_PREFIX,
                ptr::null(),
                ptr::null_mut(),
                &mut dw_size,
            ) {
                Foundation::ERROR_BUFFER_OVERFLOW => {}
                _ => panic!(),
            }

            let ptr = libc::malloc(dw_size as usize);
            if ptr.is_null() {
                return Err(OsError::last());
            }
            let addrs = IpAdapterAddresses(ptr.cast());

            match IpHelper::GetAdaptersAddresses(
                WinSock::AF_UNSPEC as u32,
                IpHelper::GAA_FLAG_INCLUDE_PREFIX,
                ptr::null(),
                addrs.0,
                &mut dw_size,
            ) {
                Foundation::ERROR_SUCCESS => Ok(addrs),
                _ => Err(OsError::last()),
            }
        }
    }
}

pub struct Ifaces {
    addrs: IpAdapterAddresses,
}

impl Ifaces {
    pub fn new() -> Result<Self, OsError> {
        let addrs = IpAdapterAddresses::new()?;
        Ok(Self { addrs: addrs })
    }

    pub fn iter(&self) -> IfaceIter<'_> {
        IfaceIter(Some(unsafe { &*self.addrs.0 }), None)
    }
}
