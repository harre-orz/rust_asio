use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct IfaceIdx {
    ifi: libc::c_uint,
}

impl IfaceIdx {
    pub const unsafe fn from_raw(ifi: libc::c_uint) -> Self {
        Self { ifi: ifi }
    }

    pub const fn as_raw(&self) -> u32 {
        self.ifi
    }
}

#[derive(Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Clone)]
pub struct MacAddr {
    bytes: [u8; 6],
}

impl MacAddr {
    pub const fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> Self {
        Self {
            bytes: [a, b, c, d, e, f]
        }
    }

    pub const fn octets(&self) -> &[u8; 6] {
        &self.bytes
    }
}

impl From<[u8; 6]> for MacAddr {
    fn from(bytes: [u8; 6]) -> Self {
        Self {
            bytes: bytes
        }
    }
}

impl fmt::Display for MacAddr {
    #[cfg(unix)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.bytes[0],
            self.bytes[1],
            self.bytes[2],
            self.bytes[3],
            self.bytes[4],
            self.bytes[5]
        )
    }

    #[cfg(windows)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}-{:02X}-{:02X}-{:02X}-{:02X}-{:02X}",
            self.bytes[0],
            self.bytes[1],
            self.bytes[2],
            self.bytes[3],
            self.bytes[4],
            self.bytes[5]
        )
    }
}

impl fmt::Debug for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{IfaceAddrRef, IfaceIter, IfaceRef, Ifaces};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{IfaceAddrRef, IfaceIter, IfaceRef, Ifaces, PseudoPhysicalRef};
