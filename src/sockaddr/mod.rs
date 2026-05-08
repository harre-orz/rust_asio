use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::{mem, ptr, slice};

#[cfg(unix)]
use libc::{
    sa_family_t, sockaddr, sockaddr_in, sockaddr_in6, sockaddr_storage, sockaddr_un, socklen_t,
};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::{
    ADDRESS_FAMILY as sa_family_t, SOCKADDR as sockaddr, SOCKADDR_IN as sockaddr_in,
    SOCKADDR_IN6 as sockaddr_in6, SOCKADDR_STORAGE as sockaddr_storage, SOCKADDR_UN as sockaddr_un,
    socklen_t,
};

/// An alias for `libc::socklen_t`.
pub type SockLen = socklen_t;

/// The domain argument of the socket.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct AddressFamily(sa_family_t);

impl AddressFamily {
    /// Creates an OS-dependent address family.
    pub const unsafe fn from_raw(family: sa_family_t) -> Self {
        Self(family)
    }
}

impl Into<i32> for AddressFamily {
    fn into(self) -> i32 {
        self.0 as i32
    }
}

/// An abstract type such as `libc::sockaddr` and `WinSock::SOCKADDR`.
pub trait SockAddr: Copy {
    /// Initialize an indeterminate `SockAddr`.
    unsafe fn init(sa: MaybeUninit<Self>, sa_len: SockLen) -> SockAddrWithLen<Self>;

    /// Returns a raw pointer.
    fn as_raw_ptr(&self) -> *const sockaddr {
        ptr::from_ref(self).cast()
    }

    fn address_family(&self) -> AddressFamily {
        AddressFamily(unsafe { &*self.as_raw_ptr() }.sa_family)
    }
}

/// The wraps `SockAddr*` and `SockLen`.
///
/// In the case of BSD-based OS, it is equivalent to the size of `SockAddr*`.
pub struct SockAddrWithLen<S: SockAddr> {
    sa: S,
    #[cfg(not(target_os = "macos"))]
    sa_len: SockLen,
}

impl<S> SockAddrWithLen<S>
where
    S: SockAddr,
{
    pub(crate) const fn new_unchecked(sa: S, sa_len: SockLen) -> Self {
        Self {
            sa: sa,
            #[cfg(not(target_os = "macos"))]
            sa_len: sa_len,
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) const fn unwrap(self) -> (S, SockLen) {
        (self.sa, self.sa_len)
    }

    #[cfg(target_os = "macos")]
    pub const fn unwrap(self) -> (S, ()) {
        (self.sa, ())
    }
}

#[derive(Copy, Clone)]
union Inner {
    sin: sockaddr_in,
    sin6: sockaddr_in6,
}

/// An data type `sockaddr_in` or `sockaddr_in6`.
#[derive(Clone, Copy)]
pub struct SockAddrIp {
    inner: Inner,
}

impl SockAddrIp {
    pub(crate) const unsafe fn as_bytes_unchecked(&self, sa_len: SockLen) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.inner.sin).cast(), sa_len as usize) }
    }

    pub(crate) const fn is_v4(&self) -> bool {
        unsafe { self.inner.sin.sin_family == AddressFamily::AF_INET.0 }
    }

    pub(crate) const fn port(&self) -> u16 {
        u16::from_be(unsafe { self.inner.sin.sin_port })
    }

    pub(crate) const unsafe fn as_ipv4_addr_unchecked(&self) -> &Ipv4Addr {
        unsafe { mem::transmute(&self.inner.sin.sin_addr) }
    }

    pub(crate) const unsafe fn as_ipv6_addr_unchecked(&self) -> &Ipv6Addr {
        unsafe { mem::transmute(&self.inner.sin6.sin6_addr) }
    }
}

/// An data type `sockaddr_un`.
#[derive(Copy, Clone)]
pub struct SockAddrUnix {
    sun: sockaddr_un,
}

impl SockAddrUnix {
    pub(crate) const unsafe fn as_bytes_unchecked(&self, sun_len: SockLen) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.sun).cast(), sun_len as usize) }
    }
}

/// An data type `sockaddr_storage`.
#[derive(Copy, Clone)]
pub struct SockAddrStorage {
    ss: sockaddr_storage,
}

impl SockAddrStorage {
    pub(crate) const unsafe fn as_bytes_unchecked(&self, ss_len: SockLen) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.ss).cast(), ss_len as usize) }
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;
