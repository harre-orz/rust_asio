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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
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

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{Iface, IfaceAddrRef, IfaceRef, Ifaces, IfacesIter};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::Iface;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::Iface;
