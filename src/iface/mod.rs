
#[cfg(unix)]
mod unix;
pub use self::unix::{Iface};

pub struct HwAddr {
    bytes: [u8; 6],
}

impl HwAddr {
    pub fn new(addr: [u8; 6]) -> Self {
        HwAddr { bytes: addr }
    }

    pub fn octets(&self) -> &[u8; 6] {
        &self.bytes
    }
}
