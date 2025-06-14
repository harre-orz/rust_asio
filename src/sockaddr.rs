pub(crate) use self::ffi::{SockAddr, SockAddrStorage, SockAddrUnix, SockAddrIp, AddressFamily};
use crate::error::OsError;
use std::ffi::OsStr;
use std::mem::{self, MaybeUninit};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::ptr;
use std::slice;

#[cfg(target_os = "linux")]
pub mod ffi {
    use super::*;

    #[derive(Copy, Clone)]
    pub(super) union Inner {
        pub(super) sa: libc::sockaddr,
        pub(super)  sin: libc::sockaddr_in,
        pub(super)  sin6: libc::sockaddr_in6,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
    pub struct AddressFamily(pub(crate) u16);

    impl AddressFamily {
        pub const UNIX: Self = Self(libc::AF_UNIX as u16);
        pub const INET: Self = Self(libc::AF_INET as u16);
        pub const INET6: Self = Self(libc::AF_INET6 as u16);
        pub const UNSPEC: Self = Self(libc::AF_UNSPEC as u16);
    }

    pub trait SockAddr: Sized {
        const MAX_SIZE: libc::socklen_t = size_of::<Self>() as libc::socklen_t;

        unsafe fn init(sa: MaybeUninit<Self>, len: libc::socklen_t) -> Self;

        fn len(&self) -> libc::socklen_t;

        fn as_ptr(&self) -> *const libc::sockaddr {
            ptr::from_ref(self).cast()
        }
    }

    #[derive(Clone, Copy)]
    pub struct SockAddrIp {
        pub(super) sa: Inner,
        len: libc::socklen_t,
    }

    impl SockAddrIp {
        pub const fn v4(addr: Ipv4Addr, port: u16) -> Self {
            Self {
                sa: Inner {
                    sin: libc::sockaddr_in {
                        sin_family: AddressFamily::INET.0,
                        sin_port: port.to_be(),
                        sin_addr: unsafe { mem::transmute(addr) },
                        sin_zero: [0; 8],
                    },
                },
                len: 16,
            }
        }

        pub const fn v6(addr: Ipv6Addr, port: u16, scope_id: u32) -> Self {
            Self {
                sa: Inner {
                    sin6: libc::sockaddr_in6 {
                        sin6_family: AddressFamily::INET6.0,
                        sin6_port: port.to_be(),
                        sin6_addr: unsafe { mem::transmute(addr) },
                        sin6_flowinfo: 0,
                        sin6_scope_id: scope_id,
                    },
                },
                len: 28,
            }
        }

        pub const fn as_bytes(&self) -> &[u8] {
            unsafe {
                let sa = self as *const _ as *const u8;
                slice::from_raw_parts(sa, self.len as usize)
            }
        }
    }

    impl SockAddr for SockAddrIp {
        unsafe fn init(sa: MaybeUninit<Self>, len: libc::socklen_t) -> Self {
            assert!(len < Self::MAX_SIZE);

            let mut sa = sa.assume_init();
            sa.len = len;
            sa
        }

        fn len(&self) -> libc::socklen_t {
            self.len
        }
    }

    #[derive(Copy, Clone)]
    pub struct SockAddrUnix {
        pub(super) sun: libc::sockaddr_un,
        len: libc::socklen_t,
    }

    pub(super) const MAX_SUN_PATH: usize = 108;

    impl SockAddrUnix {
        pub(super) const unsafe fn new_unchecked(bytes: &[u8], off: libc::socklen_t) -> Self {
            let mut sun_path = [0i8; MAX_SUN_PATH];
            let mut i = off as usize;
            while i < sun_path.len() {
                sun_path[i] = bytes[i] as i8;
                i = i + 1;
            }
            let sun = libc::sockaddr_un {
                sun_family: AddressFamily::UNIX.0,
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

            let mut sun = sun.assume_init();
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
                    sa.sa_family = family_type.0;
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

            let mut ss = ss.assume_init();
            ss.len = len;
            ss
        }
    }
}

#[cfg(target_os = "macos")]
pub mod ffi {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
    pub struct AddressFamily(pub(super) u8);

    impl AddressFamily {
        pub const UNIX: Self = Self(libc::AF_UNIX as u8);
        pub const INET: Self = Self(libc::AF_INET as u8);
        pub const INET6: Self = Self(libc::AF_INET6 as u8);
        pub const UNSPEC: Self = Self(libc::AF_UNSPEC as u8);
    }

    pub trait SockAddr: Sized {
        const MAX_SIZE: libc::socklen_t = size_of::<Self>() as libc::socklen_t;

        unsafe fn init(sa: MaybeUninit<Self>, len: libc::socklen_t) -> Self;

        fn len(&self) -> libc::socklen_t {
            unsafe { &*self.as_ptr() }.sa_len as libc::socklen_t
        }

        fn as_ptr(&self) -> *const libc::sockaddr {
            ptr::from_ref(self).cast()
        }
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
                        sin_family: AddressFamily::INET.0,
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
                        sin6_family: AddressFamily::INET6.0,
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

    pub(super) const MAX_SUN_PATH: usize = 104;

    impl SockAddrUnix {
        pub(super) const unsafe fn new_unchecked(bytes: &[u8], off: usize) -> Self {
            let mut sun_path = [0i8; MAX_SUN_PATH];
            let mut i = off;
            while i < sun_path.len() {
                sun_path[i] = bytes[i] as i8;
                i = i + 1;
            }
            let sun = libc::sockaddr_un {
                sun_len: bytes.len() as u8 + off as u8 + 2,
                sun_family: AddressFamily::UNIX.0,
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
                    sa.sa_family = family_type.0;
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
}

#[cfg(target_os = "windows")]
pub mod ffi {
    use super::*;
    use windows_sys::Win32::Networking::WinSock;

    pub trait SockAddr: Sized {
        const MAX_SIZE: WinSock::socklen_t = size_of::<Self>() as WinSock::socklen_t;

        unsafe fn init(sa: MaybeUninit<Self>, len: WinSock::socklen_t) -> Self;

        fn len(&self) -> WinSock::socklen_t;

        fn as_ptr(&self) -> *const WinSock::SOCKADDR {
            ptr::from_ref(self).cast()
        }
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
    pub struct AddressFamily(pub(crate) WinSock::ADDRESS_FAMILY);

    impl AddressFamily {
        pub const UNIX: Self = Self(WinSock::AF_UNIX);
        pub const INET: Self = Self(WinSock::AF_INET);
        pub const INET6: Self = Self(WinSock::AF_INET6);
        pub const UNSPEC: Self = Self(WinSock::AF_UNSPEC);
    }

    #[derive(Copy, Clone)]
    pub(super) union Inner {
        pub sa: WinSock::SOCKADDR,
        pub sin: WinSock::SOCKADDR_IN,
        pub sin6: WinSock::SOCKADDR_IN6,
    }

    #[derive(Clone, Copy)]
    pub struct SockAddrIp {
        pub(super) sa: Inner,
        len: WinSock::socklen_t,
    }

    impl SockAddrIp {
        pub const fn v4(addr: Ipv4Addr, port: u16) -> Self {
            Self {
                sa: Inner {
                    sin: WinSock::SOCKADDR_IN {
                        sin_family: AddressFamily::INET.0,
                        sin_port: port.to_be(),
                        sin_addr: unsafe { mem::transmute(addr) },
                        sin_zero: [0; 8],
                    },
                },
                len: 16,
            }
        }

        pub const fn v6(addr: Ipv6Addr, port: u16, scope_id: u32) -> Self {
            Self {
                sa: Inner {
                    sin6: WinSock::SOCKADDR_IN6 {
                        sin6_family: AddressFamily::INET6.0,
                        sin6_port: port.to_be(),
                        sin6_addr: unsafe { mem::transmute(addr) },
                        sin6_flowinfo: 0,
                        Anonymous: WinSock::SOCKADDR_IN6_0 {
                            sin6_scope_id: scope_id,
                        },
                    },
                },
                len: 28,
            }
        }

        pub const fn as_bytes(&self) -> &[u8] {
            unsafe {
                let sa = self as *const _ as *const u8;
                slice::from_raw_parts(sa, self.len as usize)
            }
        }
    }

    impl SockAddr for SockAddrIp {
        unsafe fn init(sa: MaybeUninit<Self>, len: WinSock::socklen_t) -> Self {
            assert!(len < Self::MAX_SIZE);

            let mut sa = sa.assume_init();
            sa.len = len;
            sa
        }

        fn len(&self) -> WinSock::socklen_t {
            self.len
        }
    }

    #[derive(Copy, Clone)]
    pub struct SockAddrUnix {
        pub(super) sun: WinSock::SOCKADDR_UN,
        len: WinSock::socklen_t,
    }

    pub(super) const MAX_SUN_PATH: usize = 108;

    impl SockAddrUnix {
        pub(super) const unsafe fn new_unchecked(bytes: &[u8], off: WinSock::socklen_t) -> Self {
            let mut sun_path = [0i8; MAX_SUN_PATH];
            let mut i = off as usize;
            while i < sun_path.len() {
                sun_path[i] = bytes[i] as i8;
                i = i + 1;
            }
            let sun = WinSock::SOCKADDR_UN {
                sun_family: AddressFamily::UNIX.0,
                sun_path: sun_path,
            };
            SockAddrUnix {
                sun: sun,
                len: bytes.len() as WinSock::socklen_t + off + 2,
            }
        }

        pub(super) const fn len(&self) -> WinSock::socklen_t {
            self.len
        }

        pub const fn as_bytes(&self) -> &[u8] {
            let sa = ptr::from_ref(&self.sun).cast();
            let len = self.len as usize;
            unsafe { slice::from_raw_parts(sa, len) }
        }
    }

    impl SockAddr for SockAddrUnix {
        fn len(&self) -> WinSock::socklen_t {
            self.len
        }

        unsafe fn init(sun: MaybeUninit<Self>, len: WinSock::socklen_t) -> Self {
            assert!(len < Self::MAX_SIZE);

            let mut sun = sun.assume_init();
            sun.len = len;
            sun
        }
    }

    #[derive(Copy, Clone)]
    pub struct SockAddrStorage {
        pub(super) ss: WinSock::SOCKADDR_STORAGE,
        len: WinSock::socklen_t,
    }

    impl SockAddrStorage {
        pub fn new(family_type: AddressFamily, bytes: &[u8]) -> Option<Self> {
            if bytes.len() + 2 < Self::MAX_SIZE as usize {
                let mut ss = MaybeUninit::<WinSock::SOCKADDR_STORAGE>::uninit();
                unsafe {
                    let sa = &mut *ss.as_mut_ptr().cast::<libc::sockaddr>();
                    sa.sa_family = family_type.0;
                    ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        ptr::from_mut(&mut sa.sa_data).cast(),
                        bytes.len(),
                    );
                    Some(Self {
                        ss: ss.assume_init(),
                        len: bytes.len() as WinSock::socklen_t + 2,
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
        fn len(&self) -> WinSock::socklen_t {
            self.len
        }

        unsafe fn init(ss: MaybeUninit<Self>, len: WinSock::socklen_t) -> Self {
            assert!(len < Self::MAX_SIZE);

            let mut ss = ss.assume_init();
            ss.len = len;
            ss
        }
    }
}

impl SockAddrIp {
    pub const fn family_type(&self) -> AddressFamily {
        AddressFamily(unsafe { self.sa.sa.sa_family })
    }

    pub const unsafe fn as_ipv4_addr(&self) -> &Ipv4Addr {
        mem::transmute(&self.sa.sin.sin_addr)
    }

    pub const unsafe fn as_ipv6_addr(&self) -> &Ipv6Addr {
        mem::transmute(&self.sa.sin6.sin6_addr)
    }

    pub const fn port(&self) -> u16 {
        u16::from_be(unsafe { self.sa.sin.sin_port })
    }
}

impl SockAddrUnix {
    pub const fn family_type(&self) -> AddressFamily {
        AddressFamily::UNIX
    }

    pub fn new_path(path: &Path) -> Result<Self, OsError> {
        let path = path.as_os_str().as_encoded_bytes();
        if path.len() < ffi::MAX_SUN_PATH {
            Ok(unsafe { Self::new_unchecked(path, 0) })
        } else {
            Err(OsError::NAME_TOO_LONG)
        }
    }

    pub fn new_abstract(name: &str) -> Result<Self, OsError> {
        let name = name.as_bytes();
        if name.len() + 1 < ffi::MAX_SUN_PATH {
            Ok(unsafe { Self::new_unchecked(name, 1) })
        } else {
            Err(OsError::NAME_TOO_LONG)
        }
    }

    pub const fn new_unnamed() -> Self {
        unsafe { Self::new_unchecked(&[], 1) }
    }

    pub fn as_path(&self) -> Option<&Path> {
        if self.sun.sun_path[2] == 0 {
            None
        } else {
            let bytes = &self.sun.sun_path[2..self.len() as usize];
            unsafe {
                let bytes = slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len());
                Some(Path::new(OsStr::from_encoded_bytes_unchecked(bytes)))
            }
        }
    }

    pub fn as_abstract(&self) -> Option<&str> {
        if self.sun.sun_path[2] != 0 || self.len() == 2 {
            None
        } else {
            let bytes = &self.sun.sun_path[3..self.len() as usize];
            let bytes = unsafe { slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len()) };
            str::from_utf8(bytes).ok()
        }
    }

    pub const fn is_unnamed(&self) -> bool {
        self.as_bytes().len() == 2
    }
}

impl SockAddrStorage {
    pub const fn family_type(&self) -> AddressFamily {
        AddressFamily(self.ss.ss_family)
    }
}
