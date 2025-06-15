use crate::error::OsError;
use crate::socket_base::{AddressFamily, SockAddr};
use std::ffi::OsStr;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::{fmt, mem, slice};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{SockAddrIp, SockAddrStorage, SockAddrUnix};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{SockAddrIp, SockAddrStorage, SockAddrUnix};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::{SockAddrIp, SockAddrStorage, SockAddrUnix};

impl SockAddrIp {
    pub const fn family_type(&self) -> AddressFamily {
        unsafe { AddressFamily::new_unchecked(self.sa.sa.sa_family) }
    }

    pub const unsafe fn as_ipv4_addr(&self) -> &Ipv4Addr {
        unsafe { mem::transmute(&self.sa.sin.sin_addr) }
    }

    pub const unsafe fn as_ipv6_addr(&self) -> &Ipv6Addr {
        unsafe { mem::transmute(&self.sa.sin6.sin6_addr) }
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
        if path.len() < Self::MAX_SUN_PATH {
            Ok(unsafe { Self::new_unchecked(path, 0) })
        } else {
            Err(OsError::NAME_TOO_LONG)
        }
    }

    pub fn new_abstract(name: &str) -> Result<Self, OsError> {
        let name = name.as_bytes();
        if name.len() + 1 < Self::MAX_SUN_PATH {
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
        unsafe { AddressFamily::new_unchecked(self.ss.ss_family) }
    }
}

impl fmt::Debug for SockAddrIp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.family_type() {
            AddressFamily::INET => {
                let addr = unsafe { self.as_ipv4_addr() };
                write!(f, "sockaddr_in {{ addr: {}, port: {} }}", addr, self.port())
            }
            AddressFamily::INET6 => {
                let addr = unsafe { self.as_ipv6_addr() };
                let scope_id = unsafe { self.scope_id() };
                write!(
                    f,
                    "sockaddr_in6 {{ addr: {}, port: {}, scope_id: {} }}",
                    addr,
                    self.port(),
                    scope_id
                )
            }
            _ => unreachable!(),
        }
    }
}
