use super::{SockAddr, SockAddrWithLen, SockLen};
use crate::error::{OsError, Result};
use crate::iface::{EthAddr, IfaceIdx};
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::{mem, ptr, slice};

/// The domain argument of the socket.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct AddressFamily(pub(super) libc::sa_family_t);

impl AddressFamily {
    /// Local communication.
    pub const AF_LOCAL: Self = Self(libc::AF_UNIX as libc::sa_family_t);

    /// IPv4 Internet protocols.
    pub const AF_INET: Self = Self(libc::AF_INET as libc::sa_family_t);

    /// IPv6 Internet protocols.
    pub const AF_INET6: Self = Self(libc::AF_INET6 as libc::sa_family_t);

    pub const fn from_sockaddr<S>(sockaddr: &S) -> Self
    where
        S: SockAddr,
    {
        let sa = unsafe { &*(ptr::from_ref(sockaddr) as *const libc::sockaddr) };
        Self(sa.sa_family)
    }
}

const fn set_socklen(sa: *mut libc::sockaddr, sa_len: SockLen) {
    unsafe { &mut *sa }.sa_len = sa_len as u8;
}

/// An data type `sockaddr_in` or `sockaddr_in6`.
#[derive(Clone, Copy)]
pub union SockAddrIp {
    pub(super) sin: libc::sockaddr_in,
    pub(super) sin6: libc::sockaddr_in6,
}

impl SockAddrIp {
    pub(crate) const fn v4(addr: Ipv4Addr, port: u16) -> SockAddrWithLen<Self> {
        SockAddrWithLen {
            sa: Self {
                sin: libc::sockaddr_in {
                    sin_family: AddressFamily::AF_INET.0,
                    sin_len: size_of::<libc::sockaddr_in>() as u8,
                    sin_port: port.to_be(),
                    sin_addr: unsafe { mem::transmute(addr) },
                    sin_zero: [0; 8],
                },
            },
        }
    }

    pub(crate) const fn v6(addr: Ipv6Addr, port: u16, scope_id: u32) -> SockAddrWithLen<Self> {
        SockAddrWithLen {
            sa: Self {
                sin6: libc::sockaddr_in6 {
                    sin6_family: AddressFamily::AF_INET6.0,
                    sin6_len: size_of::<libc::sockaddr_in6>() as u8,
                    sin6_port: port.to_be(),
                    sin6_addr: unsafe { mem::transmute(addr) },
                    sin6_flowinfo: 0,
                    sin6_scope_id: scope_id,
                },
            },
        }
    }
}

impl SockAddr for SockAddrIp {
    unsafe fn init(mut sa: MaybeUninit<Self>, sa_len: SockLen) -> SockAddrWithLen<Self> {
        set_socklen(sa.as_mut_ptr().cast(), sa_len);
        unsafe { SockAddrWithLen::new_unchecked(sa.assume_init(), sa_len) }
    }
}

/// An data type `sockaddr_un`.
#[derive(Copy, Clone)]
pub struct SockAddrUnix {
    pub(super) sun: libc::sockaddr_un,
}

impl SockAddrUnix {
    const MAX_SUN_PATH: usize = 104;

    pub(crate) const fn new(bytes: &[u8], is_abstract: bool) -> Result<SockAddrWithLen<Self>> {
        let mut data_len = bytes.len();
        let mut sun_path: [MaybeUninit<libc::c_char>; Self::MAX_SUN_PATH] =
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
                    sun_len: 2 + data_len as u8,
                    sun_path: unsafe {
                        mem::transmute::<_, [libc::c_char; Self::MAX_SUN_PATH]>(sun_path)
                    },
                },
            },
        })
    }
}

impl SockAddr for SockAddrUnix {
    unsafe fn init(mut sa: MaybeUninit<Self>, sa_len: SockLen) -> SockAddrWithLen<Self> {
        set_socklen(sa.as_mut_ptr().cast(), sa_len);
        unsafe { SockAddrWithLen::new_unchecked(sa.assume_init(), sa_len) }
    }
}

/// An data type `sockaddr_storage`.
#[derive(Copy, Clone)]
pub struct SockAddrStorage {
    pub(super) ss: libc::sockaddr_storage,
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
            sa.sa_len = 2 + bytes.len() as u8;
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                ptr::from_mut(&mut sa.sa_data).cast(),
                bytes.len(),
            );
            Self {
                ss: ss.assume_init(),
            }
        };
        Some(SockAddrWithLen { sa: ss })
    }
}

impl SockAddr for SockAddrStorage {
    unsafe fn init(mut sa: MaybeUninit<Self>, sa_len: SockLen) -> SockAddrWithLen<Self> {
        set_socklen(sa.as_mut_ptr().cast(), sa_len);
        unsafe { SockAddrWithLen::new_unchecked(sa.assume_init(), sa_len) }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SockAddrPhysical {
    pub(super) sdl: libc::sockaddr_dl,
}

impl SockAddrPhysical {
    pub const fn iface_idx(&self) -> IfaceIdx {
        unsafe { IfaceIdx::from_raw(self.sdl.sdl_index as libc::c_uint) }
    }

    pub const fn eth_addr(&self) -> Option<&EthAddr> {
        if self.sdl.sdl_alen == 6 {
            unsafe {
                let ptr = self.sdl.sdl_data.as_ptr().add(self.sdl.sdl_nlen as usize);
                Some(mem::transmute(ptr))
            }
        } else {
            None
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.sdl).cast(), size_of_val(&self.sdl)) }
    }
}

impl SockAddr for SockAddrPhysical {
    unsafe fn init(sa: MaybeUninit<Self>, sa_len: SockLen) -> SockAddrWithLen<Self> {
        unsafe { SockAddrWithLen::new_unchecked(sa.assume_init(), sa_len) }
    }
}

impl PartialEq for SockAddrPhysical {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for SockAddrPhysical {}
