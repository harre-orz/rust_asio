use std::fmt;

#[derive(Copy, Clone)]
pub struct EthAddr {
    bytes: [u8; 6],
}

impl EthAddr {
    pub const fn new(bytes: [u8; 6]) -> Self {
        EthAddr { bytes: bytes }
    }

    pub const fn octets(&self) -> &[u8; 6] {
        &self.bytes
    }
}

impl fmt::Debug for EthAddr {
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

impl fmt::Display for EthAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{Iface, IfaceAddrRef, IfaceRef, Ifaces, IfacesIter};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::Iface;
