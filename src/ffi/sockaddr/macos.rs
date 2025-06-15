use crate::socket_base::{AddressFamily, SockAddr};
use std::mem;
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ptr;
use std::slice;

#[derive(Copy, Clone)]
pub(super) union Inner {
    pub(super) sa: libc::sockaddr,
    pub(super) sin: libc::sockaddr_in,
    pub(super) sin6: libc::sockaddr_in6,
}

#[derive(Copy, Clone)]
pub struct SockAddrIp {
    pub(super) sa: Inner,
}

impl SockAddrIp {
    pub const fn v4(addr: Ipv4Addr, port: u16) -> Self {
        Self {
            sa: Inner {
                sin: libc::sockaddr_in {
                    sin_len: 16,
                    sin_family: AddressFamily::INET.get(),
                    sin_port: port.to_be(),
                    sin_addr: unsafe { mem::transmute(addr) },
                    sin_zero: [0; 8],
                },
            },
        }
    }

    pub const fn v6(addr: Ipv6Addr, port: u16, scope_id: u32) -> Self {
        Self {
            sa: Inner {
                sin6: libc::sockaddr_in6 {
                    sin6_len: 28,
                    sin6_family: AddressFamily::INET6.get(),
                    sin6_port: port.to_be(),
                    sin6_addr: unsafe { mem::transmute(addr) },
                    sin6_flowinfo: 0,
                    sin6_scope_id: scope_id,
                },
            },
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        unsafe {
            let sa = ptr::from_ref(&self.sa.sa).cast();
            let len = self.sa.sa.sa_len as usize;
            slice::from_raw_parts(sa, len)
        }
    }

    pub const unsafe fn scope_id(&self) -> u32 {
        self.sa.sin6.sin6_scope_id
    }
}

impl SockAddr for SockAddrIp {
    unsafe fn init(sa: MaybeUninit<Self>, len: libc::socklen_t) -> Self {
        assert!(len <= Self::MAX_SIZE);

        let mut sa = sa.assume_init();
        sa.sa.sa.sa_len = len as u8;
        sa
    }
}

#[derive(Copy, Clone)]
pub struct SockAddrUnix {
    pub(super) sun: libc::sockaddr_un,
}

impl SockAddrUnix {
    pub(super) const MAX_SUN_PATH: usize = 104;

    pub(super) const unsafe fn new_unchecked(bytes: &[u8], off: usize) -> Self {
        let mut sun_path = [0i8; Self::MAX_SUN_PATH];
        let mut i = off;
        while i < sun_path.len() {
            sun_path[i] = bytes[i] as i8;
            i = i + 1;
        }
        let sun = libc::sockaddr_un {
            sun_len: bytes.len() as u8 + off as u8 + 2,
            sun_family: AddressFamily::UNIX.get(),
            sun_path: sun_path,
        };
        SockAddrUnix { sun: sun }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        let sa = ptr::from_ref(&self.sun).cast();
        let len = self.sun.sun_len as usize;
        unsafe { slice::from_raw_parts(sa, len) }
    }
}

impl SockAddr for SockAddrUnix {
    unsafe fn init(sun: MaybeUninit<Self>, len: libc::socklen_t) -> Self {
        assert!(len < Self::MAX_SIZE);

        let mut sun = sun.assume_init();
        sun.sun.sun_len = len as u8;
        sun
    }
}

#[derive(Copy, Clone)]
pub struct SockAddrStorage {
    pub(super) ss: libc::sockaddr_storage,
}

impl SockAddrStorage {
    pub fn new(family_type: AddressFamily, bytes: &[u8]) -> Option<Self> {
        if bytes.len() + 2 < Self::MAX_SIZE as usize {
            let mut ss = MaybeUninit::<libc::sockaddr_storage>::uninit();
            unsafe {
                let sa = &mut *ss.as_mut_ptr().cast::<libc::sockaddr>();
                sa.sa_len = bytes.len() as u8 + 2;
                sa.sa_family = family_type.get();
                ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    ptr::from_mut(&mut sa.sa_data).cast(),
                    bytes.len(),
                );
                Some(Self {
                    ss: ss.assume_init(),
                })
            }
        } else {
            None
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        let ss = ptr::from_ref(&self.ss).cast();
        let len = self.ss.ss_len as usize;
        unsafe { slice::from_raw_parts(ss, len) }
    }
}

impl SockAddr for SockAddrStorage {
    unsafe fn init(sun: MaybeUninit<Self>, len: libc::socklen_t) -> Self {
        assert!(len < Self::MAX_SIZE);

        let mut ss = sun.assume_init();
        ss.ss.ss_len = len as u8;
        ss
    }
}
