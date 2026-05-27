use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::{mem, ptr, slice};

/// An alias for `libc::socklen_t`.
#[cfg(unix)]
pub type SockLen = libc::socklen_t;

/// An alias for `windows_sys::Win32::Networking::WinSock::socklen_t`.
#[cfg(windows)]
pub type SockLen = windows_sys::Win32::Networking::WinSock::socklen_t;

impl AddressFamily {
    pub(crate) const UNSPEC: AddressFamily = AddressFamily(0);
}

impl Into<i32> for AddressFamily {
    fn into(self) -> i32 {
        self.0 as i32
    }
}

/// An abstract type such as `libc::sockaddr` and `WinSock::SOCKADDR`.
pub trait SockAddr: Copy + 'static {
    /// Initialize an indeterminate `SockAddr`.
    unsafe fn init(sa: MaybeUninit<Self>, sa_len: SockLen) -> SockAddrWithLen<Self>;
}

/// The wraps `SockAddr*` and `SockLen`.
///
/// In the case of BSD-based OS, it is equivalent to the size of `SockAddr*`.
#[derive(Copy, Clone)]
pub struct SockAddrWithLen<S: SockAddr> {
    pub(crate) sa: S,
    #[cfg(not(target_os = "macos"))]
    sa_len: SockLen,
}

impl<S> SockAddrWithLen<S>
where
    S: SockAddr,
{
    pub(crate) const unsafe fn new_unchecked(sa: S, _sa_len: SockLen) -> Self {
        Self {
            sa: sa,
            #[cfg(not(target_os = "macos"))]
            sa_len: _sa_len,
        }
    }

    pub const fn len(&self) -> SockLen {
        #[cfg(not(target_os = "macos"))]
        let len = self.sa_len;
        #[cfg(target_os = "macos")]
        let len = unsafe { &*(ptr::from_ref(&self.sa) as *const libc::sockaddr) }.sa_len as SockLen;
        len
    }

    pub const unsafe fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.sa).cast(), self.len() as usize) }
    }
}

impl SockAddrIp {
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

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{AddressFamily, SockAddrIp, SockAddrPhysical, SockAddrStorage, SockAddrUnix};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{AddressFamily, SockAddrIp, SockAddrPhysical, SockAddrStorage, SockAddrUnix};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{AddressFamily, SockAddrIp, SockAddrStorage, SockAddrUnix};
