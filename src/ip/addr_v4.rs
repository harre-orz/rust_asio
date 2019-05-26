//

use super::IpAddr;
use std::fmt;
use std::net::Ipv4Addr;
use std::ops::{AddAssign, SubAssign};

/// The IP address of version-4.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IpAddrV4 {
    pub(super) bytes: [u8; 4],
}

impl IpAddrV4 {
    /// Constructs a new IP address of version-4.
    /// The result will represent the IP address `a`.`b`.`c`.`d`.
    ///
    /// # Example
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::new(192,168,0,1);
    /// assert_eq!(format!("{}", ip), "192.168.0.1");
    /// ```
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        IpAddrV4 { bytes: [a, b, c, d] }
    }

    /// Constructs a loopback IP address of version-4.
    /// The result will represent the IP address `127`.`0`.`0`.`1`.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::loopback();
    /// assert_eq!(ip, IpAddrV4::new(127,0,0,1));
    /// ```
    pub const fn loopback() -> Self {
        IpAddrV4::new(127, 0, 0, 1)
    }

    /// Alters into a C-style data.
    pub const fn into_in_addr(self) -> libc::in_addr {
        libc::in_addr {
            s_addr: self.to_u32().to_be(),
        }
    }

    /// Returns true if self is an unspecified address.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::default().is_unspecified());
    /// ```
    pub const fn is_unspecified(&self) -> bool {
        (self.bytes[0] == 0) & (self.bytes[1] == 0) & (self.bytes[2] == 0) & (self.bytes[3] == 0)
    }

    /// Return true if self is a loopback address.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::loopback().is_loopback());
    /// ```
    pub const fn is_loopback(&self) -> bool {
        (self.bytes[0] == 127) & (self.bytes[1] == 0) & (self.bytes[2] == 0) & (self.bytes[3] == 1)
    }

    /// Returns true if self is a class A address.
    ///
    /// The class A address ranges:
    ///
    /// - 10.0.0.0/8
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(10,0,0,1).is_class_a());
    /// ```
    pub const fn is_class_a(&self) -> bool {
        self.bytes[0] & 0x80 == 0
    }

    /// Returns true if self is a class B address.
    ///
    /// The class B address ranges:
    ///
    /// - 172.16.0.0/12
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(172,16,0,1).is_class_b());
    /// ```
    pub const fn is_class_b(&self) -> bool {
        self.bytes[0] & 0xC0 == 0x80
    }

    /// Returns true if self is a class C address.
    ///
    /// The class c address ranges:
    ///
    /// - 192.168.0.0/16
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(192,168,0,1).is_class_c());
    /// ```
    pub const fn is_class_c(&self) -> bool {
        self.bytes[0] & 0xE0 == 0xC0
    }

    /// Returns true if self is a private address.
    ///
    /// The private address ranges:
    ///
    ///  - 10.0.0.0/8
    ///  - 172.16.0.0/12
    ///  - 192.168.0.0/16
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(192,168,0,1).is_private());
    /// ```
    pub const fn is_private(&self) -> bool {
        self.is_class_a() | self.is_class_b() | self.is_class_c()
    }

    /// Returns true if self is a class D address.
    ///
    /// The class D address ranges:
    ///
    /// - 224.0.0.0/4
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(224,0,0,1).is_multicast());
    /// ```
    pub const fn is_multicast(&self) -> bool {
        self.bytes[0] & 0xF0 == 0xE0
    }

    /// Returns true if self is a link-local address.
    ///
    /// The link-local address ranges:
    ///
    /// - 169.254.0.0/16
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(169,254,0,0).is_link_local());
    /// ```
    pub const fn is_link_local(&self) -> bool {
        (self.bytes[0] == 0xA9) & (self.bytes[1] == 0xFE)
    }

    /// Return 4 bytes.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::new(169,254,0,1);
    /// assert_eq!(ip.octets(), &[169,254,0,1]);
    /// ```
    pub const fn octets(&self) -> &[u8; 4] {
        &self.bytes
    }

    /// Returns `u32` in host byte order.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV4;
    ///
    /// assert_eq!(IpAddrV4::new(10,0,0,1).to_u32(), 10*256*256*256+1);
    /// ```
    pub const fn to_u32(&self) -> u32 {
        ((self.bytes[0] as u32) << 24)
            | ((self.bytes[1] as u32) << 16)
            | ((self.bytes[2] as u32) << 8)
            | ((self.bytes[3] as u32) << 0)
    }
}

/// Constructs an unspecified IP address of version-4.
impl Default for IpAddrV4 {
    fn default() -> Self {
        IpAddrV4::new(0, 0, 0, 0)
    }
}

impl PartialEq<IpAddr> for IpAddrV4 {
    fn eq(&self, other: &IpAddr) -> bool {
        other.eq(self)
    }
}

impl AddAssign<i64> for IpAddrV4 {
    fn add_assign(&mut self, rhs: i64) {
        *self = Self::from(self.to_u32() + rhs as u32)
    }
}

impl SubAssign<i64> for IpAddrV4 {
    fn sub_assign(&mut self, rhs: i64) {
        *self = Self::from(self.to_u32() - rhs as u32);
    }
}

impl fmt::Display for IpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let &[a, b, c, d] = &self.bytes;
        write!(f, "{}.{}.{}.{}", a, b, c, d)
    }
}

impl From<u32> for IpAddrV4 {
    fn from(addr: u32) -> Self {
        IpAddrV4 {
            bytes: [
                ((addr & 0xFF000000) >> 24) as u8,
                ((addr & 0x00FF0000) >> 16) as u8,
                ((addr & 0x0000FF00) >> 8) as u8,
                ((addr & 0x000000FF) >> 0) as u8,
            ],
        }
    }
}

impl From<&Ipv4Addr> for IpAddrV4 {
    fn from(addr: &Ipv4Addr) -> Self {
        IpAddrV4 { bytes: addr.octets() }
    }
}

impl From<libc::in_addr> for IpAddrV4 {
    fn from(addr: libc::in_addr) -> Self {
        Self::from(u32::from_be(addr.s_addr))
    }
}

#[test]
fn test_default() {
    let ip = IpAddrV4::default();
    assert_eq!(ip, IpAddrV4::new(0, 0, 0, 0));
}

#[test]
fn test_bytes() {
    assert_eq!(IpAddrV4::default().bytes, [0, 0, 0, 0]);
    assert_eq!(IpAddrV4::new(1, 2, 3, 4).bytes, [1, 2, 3, 4]);
    assert_eq!(IpAddrV4::new(1, 2, 3, 4), IpAddrV4 { bytes: ([1, 2, 3, 4]) });
    assert!(IpAddrV4::new(1, 2, 3, 4) < IpAddrV4::new(1, 2, 3, 5));
    assert!(IpAddrV4::new(1, 2, 3, 4) < IpAddrV4::new(1, 2, 4, 0));
    assert!(IpAddrV4::new(1, 2, 3, 4) < IpAddrV4::new(1, 3, 0, 0));
    assert!(IpAddrV4::new(1, 2, 3, 4) < IpAddrV4::new(2, 0, 0, 0));
}

#[test]
fn test_from() {
    let addr = IpAddrV4 { bytes: [1, 2, 3, 4] };
    let octets = addr.octets();
    assert_eq!(octets, &[1, 2, 3, 4]);
}

#[test]
fn test_octets() {
    let addr = IpAddrV4::new(1, 2, 3, 4);
    assert_eq!(addr.octets(), &[1, 2, 3, 4]);
}

#[test]
fn test_display() {
    let s = format!("{}", IpAddrV4::new(1, 2, 3, 4));
    assert_eq!(s, "1.2.3.4");
}

#[test]
fn test_ord() {
    let a = IpAddrV4::new(1, 2, 3, 4);
    let b = IpAddrV4::new(1, 2, 3, 5);
    let c = IpAddrV4::new(1, 2, 4, 4);
    let d = IpAddrV4::new(1, 3, 3, 4);
    let e = IpAddrV4::new(2, 2, 3, 4);
    assert!(a < b);
    assert!(b < c);
    assert!(c < d);
    assert!(d < e);
}

#[test]
fn test_add() {
    let mut a = IpAddrV4::new(1, 2, 3, 4);
    a += 1;
    assert_eq!(a, IpAddrV4::new(1, 2, 3, 5));
    a += 2;
    assert_eq!(a, IpAddrV4::new(1, 2, 3, 7));
    a += 512;
    assert_eq!(a, IpAddrV4::new(1, 2, 5, 7));
}

#[test]
fn test_sub() {
    let mut a = IpAddrV4::new(1, 2, 3, 4);
    a -= 1;
    assert_eq!(a, IpAddrV4::new(1, 2, 3, 3));
    a -= 2;
    assert_eq!(a, IpAddrV4::new(1, 2, 3, 1));
    a -= 512;
    assert_eq!(a, IpAddrV4::new(1, 2, 1, 1));
}

#[test]
fn test_format() {
    assert_eq!(format!("{}", IpAddrV4::default()), "0.0.0.0");
    assert_eq!(format!("{}", IpAddrV4::loopback()), "127.0.0.1");
}
