use super::{
    AddressFamily, Inner, SockAddr, SockAddrIp, SockAddrStorage, SockAddrUnix, SockAddrWithLen,
    SockLen,
};
use crate::error::OsError;
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::raw::c_char;
use std::{mem, ptr};

impl AddressFamily {
    /// Local communication.
    pub const AF_LOCAL: Self = Self(libc::AF_UNIX as libc::sa_family_t);

    /// IPv4 Internet protocols.
    pub const AF_INET: Self = Self(libc::AF_INET as libc::sa_family_t);

    /// IPv6 Internet protocols.
    pub const AF_INET6: Self = Self(libc::AF_INET6 as libc::sa_family_t);

    /// Low-level packet interface.
    pub const AF_PACKET: Self = Self(libc::PF_PACKET as libc::sa_family_t);
}

impl SockAddrIp {
    pub(crate) const fn v4(addr: Ipv4Addr, port: u16) -> SockAddrWithLen<Self> {
        SockAddrWithLen {
            sa: Self {
                inner: Inner {
                    sin: libc::sockaddr_in {
                        sin_family: AddressFamily::AF_INET.0,
                        sin_port: port.to_be(),
                        sin_addr: unsafe { mem::transmute(addr) },
                        sin_zero: [0; 8],
                    },
                },
            },
            sa_len: size_of::<libc::sockaddr_in>() as SockLen,
        }
    }

    pub(crate) const fn v6(addr: Ipv6Addr, port: u16, scope_id: u32) -> SockAddrWithLen<Self> {
        SockAddrWithLen {
            sa: Self {
                inner: Inner {
                    sin6: libc::sockaddr_in6 {
                        sin6_family: AddressFamily::AF_INET6.0,
                        sin6_port: port.to_be(),
                        sin6_addr: unsafe { mem::transmute(addr) },
                        sin6_flowinfo: 0,
                        sin6_scope_id: scope_id,
                    },
                },
            },
            sa_len: size_of::<libc::sockaddr_in6>() as SockLen,
        }
    }

    pub(crate) const unsafe fn scope_id_unchecked(&self) -> u32 {
        unsafe { self.inner.sin6.sin6_scope_id }
    }
}

impl SockAddr for SockAddrIp {
    unsafe fn init(sa: MaybeUninit<Self>, sa_len: SockLen) -> SockAddrWithLen<Self> {
        unsafe { SockAddrWithLen::new_unchecked(sa.assume_init(), sa_len) }
    }
}

impl SockAddrUnix {
    const MAX_SUN_PATH: usize = 108;

    pub(crate) const fn new(
        bytes: &[u8],
        is_abstract: bool,
    ) -> Result<SockAddrWithLen<Self>, OsError> {
        let mut data_len = bytes.len();
        let mut sun_path: [MaybeUninit<c_char>; Self::MAX_SUN_PATH] =
            [const { MaybeUninit::uninit() }; Self::MAX_SUN_PATH];
        let ptr = if is_abstract {
            data_len += 1;
            sun_path[0].write(0i8);
            sun_path[1].as_mut_ptr()
        } else {
            sun_path[0].as_mut_ptr()
        };
        if data_len > sun_path.len() {
            return Err(OsError::NAME_TOO_LONG);
        }
        unsafe { ptr.copy_from_nonoverlapping(bytes.as_ptr().cast(), bytes.len()) };
        Ok(SockAddrWithLen {
            sa: Self {
                sun: libc::sockaddr_un {
                    sun_family: AddressFamily::AF_LOCAL.0,
                    sun_path: unsafe {
                        mem::transmute::<_, [c_char; Self::MAX_SUN_PATH]>(sun_path)
                    },
                },
            },
            sa_len: 2 + data_len as SockLen,
        })
    }
}

impl SockAddr for SockAddrUnix {
    unsafe fn init(sa: MaybeUninit<Self>, sa_len: SockLen) -> SockAddrWithLen<Self> {
        unsafe { SockAddrWithLen::new_unchecked(sa.assume_init(), sa_len) }
    }
}

impl SockAddrStorage {
    pub(crate) fn new(family_type: AddressFamily, bytes: &[u8]) -> Option<SockAddrWithLen<Self>> {
        if 2 + bytes.len() > size_of::<libc::sockaddr_storage>() {
            return None;
        }
        let mut ss = MaybeUninit::<libc::sockaddr_storage>::uninit();
        let ss = unsafe {
            let sa = &mut *ss.as_mut_ptr().cast::<libc::sockaddr>();
            sa.sa_family = family_type.0;
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                ptr::from_mut(&mut sa.sa_data).cast(),
                bytes.len(),
            );
            Self {
                ss: ss.assume_init(),
            }
        };
        Some(SockAddrWithLen {
            sa: ss,
            sa_len: 2 + bytes.len() as SockLen,
        })
    }
}

impl SockAddr for SockAddrStorage {
    unsafe fn init(sa: MaybeUninit<Self>, sa_len: SockLen) -> SockAddrWithLen<Self> {
        unsafe { SockAddrWithLen::new_unchecked(sa.assume_init(), sa_len) }
    }
}
