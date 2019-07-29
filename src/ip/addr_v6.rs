//

use super::{IpAddr, IpAddrV4};
use std::fmt;
use std::mem;
use std::net::Ipv6Addr;
use std::ops::{AddAssign, SubAssign};

/// The IP address of version-6.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IpAddrV6 {
    pub(super) bytes: [u8; 16],
    pub(super) scope_id: u32,
}

impl IpAddrV6 {
    /// Constructs a new IP address of version-6.
    /// The result will represent the IP address `a`:`b`:`c`:`d`:`e`:`f`:`g`:`h`.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::new(0,0,0,0,0,0,0,1);
    /// ```
    pub const fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Self {
        Self::with_scope_id(a, b, c, d, e, f, g, h, 0)
    }

    /// Constructs a new IP address of version-6 with sets the scope id.
    pub const fn with_scope_id(
        a: u16,
        b: u16,
        c: u16,
        d: u16,
        e: u16,
        f: u16,
        g: u16,
        h: u16,
        scope_id: u32,
    ) -> Self {
        IpAddrV6 {
            bytes: [
                ((a & 0xFF00) >> 8) as u8,
                (a & 0x00FF) as u8,
                ((b & 0xFF00) >> 8) as u8,
                (b & 0x00FF) as u8,
                ((c & 0xFF00) >> 8) as u8,
                (c & 0x00FF) as u8,
                ((d & 0xFF00) >> 8) as u8,
                (d & 0x00FF) as u8,
                ((e & 0xFF00) >> 8) as u8,
                (e & 0x00FF) as u8,
                ((f & 0xFF00) >> 8) as u8,
                (f & 0x00FF) as u8,
                ((g & 0xFF00) >> 8) as u8,
                (g & 0x00FF) as u8,
                ((h & 0xFF00) >> 8) as u8,
                (h & 0x00FF) as u8,
            ],
            scope_id: scope_id,
        }
    }

    /// Constructs a loopback IP address of version-6.
    ///
    /// # Examples
    /// ```
    /// use asyio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::loopback();
    /// assert_eq!(ip, IpAddrV6::new(0,0,0,0,0,0,0,1));
    /// ```
    pub const fn loopback() -> Self {
        IpAddrV6::new(0, 0, 0, 0, 0, 0, 0, 1)
    }

    /// Alters into a C-style data.
    pub const fn into_in6_addr(self) -> libc::in6_addr {
        libc::in6_addr {
            s6_addr: self.bytes,
        }
    }

    /// Returns true if self is a loopback address.
    pub const fn is_loopback(&self) -> bool {
        (self.bytes[0] == 0)
            & (self.bytes[1] == 0)
            & (self.bytes[2] == 0)
            & (self.bytes[3] == 0)
            & (self.bytes[4] == 0)
            & (self.bytes[5] == 0)
            & (self.bytes[6] == 0)
            & (self.bytes[7] == 0)
            & (self.bytes[8] == 0)
            & (self.bytes[9] == 0)
            & (self.bytes[10] == 0)
            & (self.bytes[11] == 0)
            & (self.bytes[12] == 0)
            & (self.bytes[13] == 0)
            & (self.bytes[14] == 0)
            & (self.bytes[15] == 1)
    }

    /// Returns true if self is a link-local address.
    pub const fn is_link_local(&self) -> bool {
        (self.bytes[0] == 0xFE) & ((self.bytes[1] & 0xC0) == 0x80)
    }

    /// Returns true if self is a some multicast address.
    pub const fn is_multicast(&self) -> bool {
        self.bytes[0] == 0xFF
    }

    /// Returns true if self is a multicast address of global.
    pub const fn is_multicast_global(&self) -> bool {
        (self.bytes[0] == 0xFF) & ((self.bytes[1] & 0x0F) == 0x0E)
    }

    /// Returns true if self is a multicast address of link-local.
    pub const fn is_multicast_link_local(&self) -> bool {
        (self.bytes[0] == 0xFF) & ((self.bytes[1] & 0x0F) == 0x02)
    }

    /// Returns true if self is a multicast address of node-local.
    pub const fn is_multicast_node_local(&self) -> bool {
        (self.bytes[0] == 0xFF) & ((self.bytes[1] & 0x0F) == 0x01)
    }

    /// Returns true if self is a multicast address of org-local.
    pub const fn is_multicast_org_local(&self) -> bool {
        (self.bytes[0] == 0xFF) & ((self.bytes[1] & 0x0F) == 0x08)
    }

    /// Returns true if self is a multicast address for site-local.
    pub const fn is_multicast_site_local(&self) -> bool {
        (self.bytes[0] == 0xFF) & ((self.bytes[1] & 0x0F) == 0x05)
    }

    /// Returns true if self is a site-local address.
    pub const fn is_site_local(&self) -> bool {
        (self.bytes[0] == 0xFE) & ((self.bytes[1] & 0xC0) == 0xC0)
    }

    /// Returns true if self is an unspecified address.
    pub const fn is_unspecified(&self) -> bool {
        (self.bytes[0] == 0)
            & (self.bytes[1] == 0)
            & (self.bytes[2] == 0)
            & (self.bytes[3] == 0)
            & (self.bytes[4] == 0)
            & (self.bytes[5] == 0)
            & (self.bytes[6] == 0)
            & (self.bytes[7] == 0)
            & (self.bytes[8] == 0)
            & (self.bytes[9] == 0)
            & (self.bytes[10] == 0)
            & (self.bytes[11] == 0)
            & (self.bytes[12] == 0)
            & (self.bytes[13] == 0)
            & (self.bytes[14] == 0)
            & (self.bytes[15] == 0)
    }

    /// Returns true if self is a IP version-4 compatible address.
    pub const fn is_v4_compatible(&self) -> bool {
        ((self.bytes[0] == 0)
            & (self.bytes[1] == 0)
            & (self.bytes[2] == 0)
            & (self.bytes[3] == 0)
            & (self.bytes[4] == 0)
            & (self.bytes[5] == 0)
            & (self.bytes[6] == 0)
            & (self.bytes[7] == 0)
            & (self.bytes[8] == 0)
            & (self.bytes[9] == 0)
            & (self.bytes[10] == 0)
            & (self.bytes[11] == 0)
            & !((self.bytes[12] == 0)
                & (self.bytes[13] == 0)
                & (self.bytes[14] == 0)
                & ((self.bytes[15] == 0) | (self.bytes[15] == 1))))
    }

    /// Returns true if self is a mapped IP version-4 address.
    pub const fn is_v4_mapped(&self) -> bool {
        (self.bytes[0] == 0)
            & (self.bytes[1] == 0)
            & (self.bytes[2] == 0)
            & (self.bytes[3] == 0)
            & (self.bytes[4] == 0)
            & (self.bytes[5] == 0)
            & (self.bytes[6] == 0)
            & (self.bytes[7] == 0)
            & (self.bytes[8] == 0)
            & (self.bytes[9] == 0)
            & (self.bytes[10] == 0xFF)
            & (self.bytes[11] == 0xFF)
    }

    /// Retrun 16 bytes.
    pub const fn octets(&self) -> &[u8; 16] {
        &self.bytes
    }

    /// Returns a IP address version-4 if self is a mapped IP version-4 address or IP version-4 compatible address.
    pub fn to_v4(&self) -> Option<IpAddrV4> {
        if self.is_v4_mapped() || self.is_v4_compatible() {
            Some(IpAddrV4::new(
                self.bytes[12],
                self.bytes[13],
                self.bytes[14],
                self.bytes[15],
            ))
        } else {
            None
        }
    }

    /// Converts into a mapped IP version-4 address.
    ///
    /// For example: `192.168.0.1` into `::ffff::192.168.0.1`
    pub const fn v4_mapped(addr: IpAddrV4) -> Self {
        let [a, b, c, d] = addr.bytes;
        IpAddrV6 {
            bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, a, b, c, d],
            scope_id: 0,
        }
    }

    /// Converts into a IP version-4 compatible address.
    ///
    /// For example: `192.168.0.1` into `::192.168.0.1`
    pub fn v4_compatible(addr: IpAddrV4) -> Option<Self> {
        let [a, b, c, d] = addr.bytes;
        if (a == 0) & (b == 0) & (c == 0) & ((d == 0) | (d == 1)) {
            None
        } else {
            Some(IpAddrV6 {
                bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, a, b, c, d],
                scope_id: 0,
            })
        }
    }

    /// Returns the scope id.
    pub const fn scope_id(&self) -> u32 {
        self.scope_id
    }
}

/// Returns a unspecified IP-v6 address.
///
/// # Examples
/// ```
/// use asyio::ip::IpAddrV6;
///
/// let ip = IpAddrV6::default();
/// assert_eq!(ip, IpAddrV6::new(0,0,0,0,0,0,0,0));
/// ```
impl Default for IpAddrV6 {
    fn default() -> Self {
        IpAddrV6::new(0, 0, 0, 0, 0, 0, 0, 0)
    }
}

impl PartialEq<IpAddr> for IpAddrV6 {
    fn eq(&self, other: &IpAddr) -> bool {
        other.eq(self)
    }
}

impl AddAssign<i64> for IpAddrV6 {
    fn add_assign(&mut self, mut rhs: i64) {
        if rhs < 0 {
            self.sub_assign(-rhs)
        } else {
            for it in self.bytes.iter_mut().rev() {
                let (val, car) = it.overflowing_add(rhs as u8);
                *it = val;
                rhs >>= 8;
                if car {
                    rhs += 1;
                }
            }
            if rhs > 0 {
                panic!("overflow");
            }
        }
    }
}

impl SubAssign<i64> for IpAddrV6 {
    fn sub_assign(&mut self, mut rhs: i64) {
        if rhs < 0 {
            self.add_assign(-rhs)
        } else {
            for it in self.bytes.iter_mut().rev() {
                let (val, car) = it.overflowing_sub(rhs as u8);
                *it = val;
                rhs >>= 8;
                if car {
                    rhs += 1;
                }
            }
            if rhs > 0 {
                panic!("overflow");
            }
        }
    }
}

impl fmt::Display for IpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ar: &[u16; 8] = unsafe { mem::transmute(&self.bytes) };
        let mut cnt = 0;
        let mut max_idx = 0;
        let mut max_cnt = 0;
        for (i, e) in ar.iter().enumerate() {
            if *e != 0 {
                if max_cnt < cnt {
                    max_idx = i - cnt;
                    max_cnt = cnt;
                }
                cnt = 0;
            } else {
                cnt += 1;
            }
        }
        if max_cnt < cnt {
            max_idx = ar.len() - cnt;
            max_cnt = cnt;
        }

        if max_idx == 0 && max_cnt == 0 {
            return write!(
                f,
                "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                u16::from_be(ar[0]),
                u16::from_be(ar[1]),
                u16::from_be(ar[2]),
                u16::from_be(ar[3]),
                u16::from_be(ar[4]),
                u16::from_be(ar[5]),
                u16::from_be(ar[6]),
                u16::from_be(ar[7])
            );
        }

        if max_idx == 0 {
            write!(f, ":")?;
        } else {
            for i in 0..max_idx {
                write!(f, "{:x}:", u16::from_be(ar[i]))?;
            }
        }

        if max_idx + max_cnt == 8 {
            write!(f, ":")?;
        } else {
            for i in max_idx + max_cnt..ar.len() {
                write!(f, ":{:x}", u16::from_be(ar[i]))?;
            }
        }
        Ok(())
    }
}

impl From<&Ipv6Addr> for IpAddrV6 {
    fn from(addr: &Ipv6Addr) -> Self {
        IpAddrV6 {
            bytes: addr.octets(),
            scope_id: 0,
        }
    }
}

impl From<libc::in6_addr> for IpAddrV6 {
    fn from(addr: libc::in6_addr) -> Self {
        IpAddrV6 {
            bytes: addr.s6_addr,
            scope_id: 0,
        }
    }
}

#[test]
fn test_new() {
    let ip = IpAddrV6 {
        bytes: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        scope_id: 0,
    };
    assert_eq!(
        ip,
        IpAddrV6::new(0x0001, 0x0203, 0x0405, 0x0607, 0x0809, 0x0A0B, 0x0C0D, 0x0E0F)
    );
}

#[test]
fn test_bytes() {
    assert_eq!(
        IpAddrV6::default(),
        IpAddrV6 {
            bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            scope_id: 0
        }
    );
    assert_eq!(
        IpAddrV6::new(0x0102, 0x0304, 0x0506, 0x0708, 0x090a, 0x0b0c, 0x0d0e, 0x0f10,),
        IpAddrV6 {
            bytes: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            scope_id: 0
        }
    );
}

#[test]
fn test_format() {
    assert_eq!(format!("{}", IpAddrV6::default()), "::");
    assert_eq!(format!("{}", IpAddrV6::loopback()), "::1");
    assert_eq!(
        format!("{}", IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8)),
        "1:2:3:4:5:6:7:8"
    );
    assert_eq!(
        format!("{}", IpAddrV6::new(0, 2, 3, 4, 5, 6, 7, 8)),
        "::2:3:4:5:6:7:8"
    );
    assert_eq!(
        format!("{}", IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 0)),
        "1:2:3:4:5:6:7::"
    );
    assert_eq!(
        format!("{}", IpAddrV6::new(1, 2, 3, 4, 0, 6, 7, 8)),
        "1:2:3:4::6:7:8"
    );
    assert_eq!(format!("{}", IpAddrV6::new(1, 0, 0, 0, 0, 0, 0, 8)), "1::8");
}
