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

#[derive(Clone, Copy)]
pub struct SockAddrIp {
    pub(super) sa: Inner,
}

impl SockAddrIp {
    pub const fn v4(addr: Ipv4Addr, port: u16) -> Self {
        Self {
            sa: Inner {
                sin: libc::sockaddr_in {
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
        let len = match self.family_type() {
            AddressFamily::INET => size_of::<libc::sockaddr_in>(),
            AddressFamily::INET6 => size_of::<libc::sockaddr_in6>(),
            _ => 0,
        };
        unsafe {
            let sa = ptr::from_ref(self).cast();
            slice::from_raw_parts(sa, len)
        }
    }

    pub const unsafe fn scope_id(&self) -> u32 {
        unsafe { self.sa.sin6.sin6_scope_id }
    }
}

impl SockAddr for SockAddrIp {
    unsafe fn init(sa: MaybeUninit<Self>, len: libc::socklen_t) -> Self {
        assert!(len < Self::MAX_SIZE);

        unsafe { sa.assume_init() }
    }

    fn len(&self) -> libc::socklen_t {
        self.as_bytes().len() as libc::socklen_t
    }
}

#[derive(Copy, Clone)]
pub struct SockAddrUnix {
    pub(super) sun: libc::sockaddr_un,
    len: libc::socklen_t,
}

impl SockAddrUnix {
    pub(super) const MAX_SUN_PATH: usize = 108;

    pub(super) const unsafe fn new_unchecked(bytes: &[u8], off: libc::socklen_t) -> Self {
        let mut sun_path = [0i8; Self::MAX_SUN_PATH];
        let mut i = off as usize;
        while i < sun_path.len() {
            sun_path[i] = bytes[i] as i8;
            i = i + 1;
        }
        let sun = libc::sockaddr_un {
            sun_family: AddressFamily::UNIX.get(),
            sun_path: sun_path,
        };
        SockAddrUnix {
            sun: sun,
            len: bytes.len() as libc::socklen_t + off + 2,
        }
    }

    pub(super) const fn len(&self) -> libc::socklen_t {
        self.len
    }

    pub const fn as_bytes(&self) -> &[u8] {
        let sa = ptr::from_ref(&self.sun).cast();
        let len = self.len as usize;
        unsafe { slice::from_raw_parts(sa, len) }
    }
}

impl SockAddr for SockAddrUnix {
    fn len(&self) -> libc::socklen_t {
        self.len
    }

    unsafe fn init(sun: MaybeUninit<Self>, len: libc::socklen_t) -> Self {
        assert!(len < Self::MAX_SIZE);

        let mut sun = unsafe { sun.assume_init() };
        sun.len = len;
        sun
    }
}

#[derive(Copy, Clone)]
pub struct SockAddrStorage {
    pub(super) ss: libc::sockaddr_storage,
    len: libc::socklen_t,
}

impl SockAddrStorage {
    pub fn new(family_type: AddressFamily, bytes: &[u8]) -> Option<Self> {
        if bytes.len() + 2 < Self::MAX_SIZE as usize {
            let mut ss = MaybeUninit::<libc::sockaddr_storage>::uninit();
            unsafe {
                let sa = &mut *ss.as_mut_ptr().cast::<libc::sockaddr>();
                sa.sa_family = family_type.get();
                ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    ptr::from_mut(&mut sa.sa_data).cast(),
                    bytes.len(),
                );
                Some(Self {
                    ss: ss.assume_init(),
                    len: bytes.len() as libc::socklen_t + 2,
                })
            }
        } else {
            None
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        let ss = ptr::from_ref(&self.ss).cast();
        let len = self.len as usize;
        unsafe { slice::from_raw_parts(ss, len) }
    }
}

impl SockAddr for SockAddrStorage {
    fn len(&self) -> libc::socklen_t {
        self.len
    }

    unsafe fn init(ss: MaybeUninit<Self>, len: libc::socklen_t) -> Self {
        assert!(len < Self::MAX_SIZE);

        let mut ss = unsafe { ss.assume_init() };
        ss.len = len;
        ss
    }
}
