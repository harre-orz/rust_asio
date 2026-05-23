use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::{mem, ptr, slice};

#[cfg(unix)]
use libc::{sa_family_t, sockaddr};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::{
    ADDRESS_FAMILY as sa_family_t, SOCKADDR as sockaddr, SOCKADDR_IN as sockaddr_in,
    SOCKADDR_IN6 as sockaddr_in6, SOCKADDR_STORAGE as sockaddr_storage, SOCKADDR_UN as sockaddr_un,
};

/// An alias for `libc::socklen_t`.
#[cfg(unix)]
pub type SockLen = libc::socklen_t;

/// An alias for `windows_sys::Win32::Networking::WinSock::socklen_t`.
#[cfg(windows)]
pub type SockLen = windows_sys::Win32::Networking::WinSock::socklen_t;

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
    pub(crate) fn new_unchecked(sa: S, sa_len: SockLen) -> Self {
        Self {
            sa: sa,
            #[cfg(not(target_os = "macos"))]
            sa_len: sa_len,
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn unwrap(self) -> (S, SockLen) {
        let Self { sa, sa_len } = self;
        (sa, sa_len)
    }

    #[cfg(target_os = "macos")]
    pub fn unwrap(self) -> (S, ()) {
        (self.sa, ())
    }
}

impl SockAddrIp {
    pub(crate) const unsafe fn as_bytes_unchecked(&self, sa_len: SockLen) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.sin).cast(), sa_len as usize) }
    }

    pub(crate) const fn is_v4(&self) -> bool {
        unsafe { self.sin.sin_family == AddressFamily::AF_INET.0 }
    }

    pub(crate) const fn port(&self) -> u16 {
        u16::from_be(unsafe { self.sin.sin_port })
    }

    pub(crate) const unsafe fn as_ipv4_addr_unchecked(&self) -> &Ipv4Addr {
        unsafe { mem::transmute(&self.sin.sin_addr) }
    }

    pub(crate) const unsafe fn as_ipv6_addr_unchecked(&self) -> &Ipv6Addr {
        unsafe { mem::transmute(&self.sin6.sin6_addr) }
    }
}

impl SockAddrUnix {
    pub(crate) const unsafe fn as_bytes_unchecked(&self, sun_len: SockLen) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.sun).cast(), sun_len as usize) }
    }
}

impl SockAddrStorage {
    pub(crate) const unsafe fn as_bytes_unchecked(&self, ss_len: SockLen) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.ss).cast(), ss_len as usize) }
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{SockAddrIp, SockAddrPhysical, SockAddrStorage, SockAddrUnix};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{SockAddrIp, SockAddrStorage, SockAddrUnix};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{SockAddrIp, SockAddrStorage, SockAddrUnix};
