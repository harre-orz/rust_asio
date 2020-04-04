//

use super::{IpAddrV4, IpAddrV6};
use std::fmt;
use std::net;
use std::ops::{AddAssign, SubAssign};

/// The IP address of either version-4 or version-6.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IpAddr {
    V4(IpAddrV4),
    V6(IpAddrV6),
}

impl IpAddr {
    /// Returns true if self is a loopback address.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::{IpAddr, IpAddrV4, IpAddrV6};
    ///
    /// let v4 = IpAddrV4::loopback();
    /// assert_eq!(IpAddr::V4(v4).is_loopback(), true);
    ///
    /// let v6 = IpAddrV6::v4_mapped(v4);
    /// assert_eq!(IpAddr::V6(v6).is_loopback(), false);
    /// ```
    pub fn is_loopback(&self) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr.is_loopback(),
            &IpAddr::V6(ref addr) => addr.is_loopback(),
        }
    }

    /// Returns true if self is a multicast address.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::{IpAddr, IpAddrV4, IpAddrV6};
    ///
    /// let v4 = IpAddrV4::new(224, 0, 0, 1);
    /// assert_eq!(IpAddr::V4(v4).is_multicast(), true);
    ///
    /// let v6 = IpAddrV6::new(0xff02, 0, 0, 0, 0, 0, 0, 1);
    /// assert_eq!(IpAddr::V6(v6).is_multicast(), true);
    /// ```
    pub fn is_multicast(&self) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr.is_multicast(),
            &IpAddr::V6(ref addr) => addr.is_multicast(),
        }
    }

    /// Returns true if self is an unspecified address.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::{IpAddr, IpAddrV4, IpAddrV6};
    ///
    /// let v4 = IpAddrV4::default();
    /// assert_eq!(IpAddr::V4(v4).is_unspecified(), true);
    ///
    /// let v6 = IpAddrV6::v4_mapped(v4);
    /// assert_eq!(IpAddr::V6(v6).is_unspecified(), false);
    /// ```
    pub fn is_unspecified(&self) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr.is_unspecified(),
            &IpAddr::V6(ref addr) => addr.is_unspecified(),
        }
    }

    /// Return 4 bytes if version-4, return 16 bytes if version-6.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::{IpAddr, IpAddrV4, IpAddrV6};
    ///
    /// let v4 = IpAddr::V4(IpAddrV4::loopback());
    /// assert_eq!(v4.octets(), &[127, 0, 0, 1]);
    ///
    /// let v6 = IpAddr::V6(IpAddrV6::loopback());
    /// assert_eq!(v6.octets(), &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    /// ```
    pub fn octets(&self) -> &[u8] {
        match self {
            &IpAddr::V4(ref addr) => addr.octets(),
            &IpAddr::V6(ref addr) => addr.octets(),
        }
    }
}

impl From<IpAddrV4> for IpAddr {
    fn from(addr: IpAddrV4) -> Self {
        IpAddr::V4(addr)
    }
}

impl From<IpAddrV6> for IpAddr {
    fn from(addr: IpAddrV6) -> Self {
        IpAddr::V6(addr)
    }
}

impl PartialEq<IpAddrV4> for IpAddr {
    fn eq(&self, other: &IpAddrV4) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr == other,
            _ => false,
        }
    }
}

impl PartialEq<IpAddrV6> for IpAddr {
    fn eq(&self, other: &IpAddrV6) -> bool {
        match self {
            &IpAddr::V6(ref addr) => addr == other,
            _ => false,
        }
    }
}

impl AddAssign<i64> for IpAddr {
    fn add_assign(&mut self, rhs: i64) {
        match self {
            &mut IpAddr::V4(ref mut addr) => addr.add_assign(rhs),
            &mut IpAddr::V6(ref mut addr) => addr.add_assign(rhs),
        }
    }
}

impl SubAssign<i64> for IpAddr {
    fn sub_assign(&mut self, rhs: i64) {
        match self {
            &mut IpAddr::V4(ref mut addr) => addr.sub_assign(rhs),
            &mut IpAddr::V6(ref mut addr) => addr.sub_assign(rhs),
        }
    }
}

impl fmt::Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &IpAddr::V4(ref addr) => write!(f, "{}", addr),
            &IpAddr::V6(ref addr) => write!(f, "{}", addr),
        }
    }
}

impl From<&net::Ipv4Addr> for IpAddr {
    fn from(addr: &net::Ipv4Addr) -> Self {
        IpAddr::V4(addr.into())
    }
}

impl From<&net::Ipv6Addr> for IpAddr {
    fn from(addr: &net::Ipv6Addr) -> Self {
        IpAddr::V6(addr.into())
    }
}

impl From<&net::IpAddr> for IpAddr {
    fn from(addr: &net::IpAddr) -> Self {
        match addr {
            net::IpAddr::V4(addr) => IpAddr::V4(addr.into()),
            net::IpAddr::V6(addr) => IpAddr::V6(addr.into()),
        }
    }
}

/// The link-layer address.
///
/// This is also referred to as MAC address and Hardware address.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LlAddr {
    pub(super) bytes: [u8; 6],
}

impl LlAddr {
    /// Constructs a new address.
    /// The result will represent a link-layer address `aa`:`bb`:`cc`:`dd`:`ee`:`ff`.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::LlAddr;
    ///
    /// let mac = LlAddr::new(0,0,0,0,0,0);
    /// ```
    pub const fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> LlAddr {
        LlAddr {
            bytes: [a, b, c, d, e, f],
        }
    }

    /// Return 6 bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::LlAddr;
    ///
    /// assert_eq!(LlAddr::new(1,2,3,4,5,6).as_bytes(), &[1,2,3,4,5,6]);
    /// ```
    pub const fn as_bytes(&self) -> &[u8; 6] {
        &self.bytes
    }

    /// Returns the OUI (Organizationally Unique Identifier).
    ///
    /// # Example
    ///
    /// ```
    /// use asyio::ip::LlAddr;
    ///
    /// let mac = LlAddr::new(0xaa, 0xbb, 0xcc, 0, 0, 0);
    /// assert_eq!(mac.oui(), 0xaabbcc);
    /// ```
    pub const fn oui(&self) -> i32 {
        ((self.bytes[0] as i32) << 16)
            | ((self.bytes[1] as i32) << 8)
            | ((self.bytes[2] as i32) << 0)
    }
}

impl fmt::Display for LlAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl From<[u8; 6]> for LlAddr {
    fn from(bytes: [u8; 6]) -> Self {
        LlAddr { bytes: bytes }
    }
}

#[test]
fn test_octets() {
    let v4 = IpAddr::V4(IpAddrV4::loopback());
    assert!(v4.octets() == &[127, 0, 0, 1]);

    let v6 = IpAddr::V6(IpAddrV6::loopback());
    assert!(v6.octets() == &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
}

#[test]
fn test_display() {
    let v4 = IpAddr::V4(IpAddrV4::loopback());
    assert_eq!(format!("{}", v4), "127.0.0.1");

    let v6 = IpAddr::V6(IpAddrV6::loopback());
    assert_eq!(format!("{}", v6), "::1");
}

#[test]
fn test_inner() {
    assert_eq!(LlAddr::default().bytes, [0, 0, 0, 0, 0, 0]);
    assert_eq!(LlAddr::new(1, 2, 3, 4, 5, 6).bytes, [1, 2, 3, 4, 5, 6]);
    assert!(LlAddr::new(1, 2, 3, 4, 5, 6) == LlAddr::from([1, 2, 3, 4, 5, 6]));
    assert!(LlAddr::new(1, 2, 3, 4, 5, 6) < LlAddr::new(1, 2, 3, 4, 5, 7));
    assert!(LlAddr::new(1, 2, 3, 4, 5, 6) < LlAddr::new(1, 2, 3, 4, 6, 0));
    assert!(LlAddr::new(1, 2, 3, 4, 5, 6) < LlAddr::new(1, 2, 3, 5, 0, 0));
    assert!(LlAddr::new(1, 2, 3, 4, 5, 6) < LlAddr::new(1, 2, 4, 0, 0, 0));
    assert!(LlAddr::new(1, 2, 3, 4, 5, 6) < LlAddr::new(1, 3, 0, 0, 0, 0));
    assert!(LlAddr::new(1, 2, 3, 4, 5, 6) < LlAddr::new(2, 0, 0, 0, 0, 0));
}

#[test]
fn test_format() {
    assert_eq!(
        format!("{}", LlAddr::new(1, 2, 3, 4, 5, 6)),
        "01:02:03:04:05:06"
    );
    assert_eq!(
        format!("{}", LlAddr::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF)),
        "AA:BB:CC:DD:EE:FF"
    );
}
