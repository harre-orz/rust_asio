#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::Iface;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::Iface;

pub struct HwAddr {
    bytes: [u8; 6],
}

impl HwAddr {
    pub const fn new(addr: [u8; 6]) -> Self {
        HwAddr { bytes: addr }
    }

    pub const fn octets(&self) -> &[u8; 6] {
        &self.bytes
    }
}
