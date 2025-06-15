use crate::error::OsError;
use crate::socket_base::{AddressFamily, SockAddr};
use std::ffi::OsStr;
use std::fmt::{self, Formatter};
use std::mem;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::slice;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{SockAddrIp, SockAddrUnix, SockAddrStorage};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{SockAddrIp, SockAddrUnix, SockAddrStorage};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
pub use self::windows::{SockAddrIp, SockAddrUnix, SockAddrStorage};
