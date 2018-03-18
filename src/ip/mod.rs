use ffi::*;
use core::IoContext;
use prelude::{Protocol, Endpoint};
use ops::Handler;

use std::io;
use std::fmt;
use std::mem;
use std::marker::PhantomData;
use std::ops::{AddAssign, SubAssign};
use std::cmp::Ordering;

use std::net;

fn add_assign(bytes: &mut [u8], mut rhs: i64) {
    if rhs < 0 {
        sub_assign(bytes, -rhs)
    } else {
        for it in bytes.iter_mut().rev() {
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

fn sub_assign(bytes: &mut [u8], mut rhs: i64) {
    if rhs < 0 {
        add_assign(bytes, -rhs)
    } else {
        for it in bytes.iter_mut().rev() {
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

fn fmt_v6(bytes: &[u8; 16], f: &mut fmt::Formatter) -> fmt::Result {
    let ar: &[u16; 8] = unsafe { mem::transmute(bytes) };
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

fn netmask_len(addr: &[u8]) -> u8 {
    if addr[0] == 0 {
        return 0;
    }

    let mut mask = 0;
    let mut it = addr.iter();
    while let Some(&n) = it.next() {
        match n {
            0b00000000 => {
                break;
            }
            0b10000000 => {
                mask += 1;
                break;
            }
            0b11000000 => {
                mask += 2;
                break;
            }
            0b11100000 => {
                mask += 3;
                break;
            }
            0b11110000 => {
                mask += 4;
                break;
            }
            0b11111000 => {
                mask += 5;
                break;
            }
            0b11111100 => {
                mask += 6;
                break;
            }
            0b11111110 => {
                mask += 7;
                break;
            }
            0b11111111 => {
                mask += 8;
            }
            _ => return 0, // for error
        }
    }
    while let Some(&n) = it.next() {
        if n != 0 {
            return 0;
        }
    }
    mask
}

/// Implements Link-layer addresses.
///
/// Also referred to as MAC address and Hardware address.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LlAddr {
    bytes: [u8; 6],
}

impl LlAddr {
    /// Returns a Link-layer address.
    ///
    /// The result will represent the LL-address a:b:c:d:e:f.
    ///
    /// # Example
    ///
    /// ```
    /// use asyncio::ip::LlAddr;
    ///
    /// let mac = LlAddr::new(0,0,0,0,0,0);
    /// ```
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> LlAddr {
        LlAddr { bytes: [a, b, c, d, e, f] }
    }

    /// Returns 6 octets bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use asyncio::ip::LlAddr;
    ///
    /// assert_eq!(LlAddr::new(1,2,3,4,5,6).as_bytes(), &[1,2,3,4,5,6]);
    /// ```
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.bytes
    }

    /// Returns a OUI (Organizationally Unique Identifier).
    ///
    /// # Example
    ///
    /// ```
    /// use asyncio::ip::LlAddr;
    ///
    /// let mac = LlAddr::new(0xaa, 0xbb, 0xcc, 0, 0, 0);
    /// assert_eq!(mac.oui(), 0xaabbcc);
    /// ```
    pub fn oui(&self) -> i32 {
        ((self.bytes[0] as i32 * 256 + self.bytes[1] as i32) * 256 + self.bytes[2] as i32)
    }
}

impl AddAssign<i64> for LlAddr {
    fn add_assign(&mut self, rhs: i64) {
        add_assign(&mut self.bytes, rhs)
    }
}

impl SubAssign<i64> for LlAddr {
    fn sub_assign(&mut self, rhs: i64) {
        sub_assign(&mut self.bytes, rhs)
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

impl fmt::Debug for LlAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<[u8; 6]> for LlAddr {
    fn from(bytes: [u8; 6]) -> Self {
        LlAddr { bytes: bytes }
    }
}

/// Implements IP version 4 style addresses.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IpAddrV4 {
    bytes: [u8; 4],
}

impl IpAddrV4 {
    /// Returns a IP-v4 address.
    ///
    /// The result will represent the IP address `a`.`b`.`c`.`d`.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::new(192,168,0,1);
    /// ```
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> IpAddrV4 {
        IpAddrV4 { bytes: [a, b, c, d] }
    }

    /// Returns a unspecified IP-v4 address.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::any();
    /// assert_eq!(ip, IpAddrV4::new(0,0,0,0));
    /// ```
    pub fn any() -> IpAddrV4 {
        IpAddrV4 { bytes: [0; 4] }
    }

    /// Returns a IP-v4 address for a loopback address.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::loopback();
    /// assert_eq!(ip, IpAddrV4::new(127,0,0,1));
    /// ```
    pub fn loopback() -> IpAddrV4 {
        IpAddrV4::new(127, 0, 0, 1)
    }

    /// Returns true for if this is a unspecified address 0.0.0.0.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::any().is_unspecified());
    /// ```
    pub fn is_unspecified(&self) -> bool {
        self.bytes.iter().all(|&x| x == 0)
    }

    /// Return true for if this is a loopback address 127.0.0.1.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::loopback().is_loopback());
    /// ```
    pub fn is_loopback(&self) -> bool {
        (self.bytes[0] & 0xFF) == 0x7F
    }

    /// Returns true for if this is a class A address.
    ///
    /// The class A address ranges:
    ///
    /// - 10.0.0.0/8
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(10,0,0,1).is_class_a());
    /// ```
    pub fn is_class_a(&self) -> bool {
        (self.bytes[0] & 0x80) == 0
    }

    /// Returns true for if this is a class B address.
    ///
    /// The class B address ranges:
    ///
    /// - 172.16.0.0/12
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(172,16,0,1).is_class_b());
    /// ```
    pub fn is_class_b(&self) -> bool {
        (self.bytes[0] & 0xC0) == 0x80
    }

    /// Returns true for if this is a class C address.
    ///
    /// The class c address ranges:
    ///
    /// - 192.168.0.0/16
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(192,168,0,1).is_class_c());
    /// ```
    pub fn is_class_c(&self) -> bool {
        (self.bytes[0] & 0xE0) == 0xC0
    }

    /// Returns true for if this is a private address.
    ///
    /// The private address ranges:
    ///
    ///  - 10.0.0.0/8
    ///  - 172.16.0.0/12
    ///  - 192.168.0.0/16
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(192,168,0,1).is_private());
    /// ```
    pub fn is_private(&self) -> bool {
        self.is_class_a() || self.is_class_b() || self.is_class_c()
    }

    /// Returns true for if this is a class D address.
    ///
    /// The class D address ranges:
    ///
    /// - 224.0.0.0/4
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(224,0,0,1).is_multicast());
    /// ```
    pub fn is_multicast(&self) -> bool {
        (self.bytes[0] & 0xF0) == 0xE0
    }

    /// Returns true for if this is a link-local address.
    ///
    /// The link-local address ranges:
    ///
    /// - 169.254.0.0/16
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(169,254,0,0).is_link_local());
    /// ```
    pub fn is_link_local(&self) -> bool {
        self.bytes[0] == 0xA9 && self.bytes[1] == 0xFE
    }

    /// Returns 4 octets bytes.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert_eq!(IpAddrV4::new(169,254,0,1).as_bytes(), &[169,254,0,1]);
    /// ```
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.bytes
    }

    /// Returns `u32` in host byte order.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// assert_eq!(IpAddrV4::new(10,0,0,1).to_u32(), 10*256*256*256+1);
    /// ```
    pub fn to_u32(&self) -> u32 {
        ((((((self.bytes[0] as u32) << 8) + self.bytes[1] as u32) << 8) +
              self.bytes[2] as u32) << 8) + self.bytes[3] as u32
    }
}

impl AddAssign<i64> for IpAddrV4 {
    fn add_assign(&mut self, rhs: i64) {
        *self = Self::from(self.to_u32() + rhs as u32);
    }
}

impl SubAssign<i64> for IpAddrV4 {
    fn sub_assign(&mut self, rhs: i64) {
        *self = Self::from(self.to_u32() - rhs as u32);
    }
}

impl fmt::Display for IpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.bytes[0],
            self.bytes[1],
            self.bytes[2],
            self.bytes[3]
        )
    }
}

impl fmt::Debug for IpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<u32> for IpAddrV4 {
    fn from(mut addr: u32) -> Self {
        let d = (addr & 0xFF) as u8;
        addr >>= 8;
        let c = (addr & 0xFF) as u8;
        addr >>= 8;
        let b = (addr & 0xFF) as u8;
        addr >>= 8;
        IpAddrV4::new(addr as u8, b, c, d)
    }
}

impl From<[u8; 4]> for IpAddrV4 {
    fn from(bytes: [u8; 4]) -> Self {
        IpAddrV4 { bytes: bytes }
    }
}

impl From<net::Ipv4Addr> for IpAddrV4 {
    fn from(ip: net::Ipv4Addr) -> Self {
        ip.octets().into()
    }
}

/// Implements IP version 6 style addresses.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IpAddrV6 {
    scope_id: u32,
    bytes: [u8; 16],
}

impl IpAddrV6 {
    /// Returns a IP-v6 address.
    ///
    /// The result will represent the IP address `a`:`b`:`c`:`d`:`e`:`f`:`g`:`h`
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::new(0,0,0,0,0,0,0,1);
    /// ```
    pub fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> IpAddrV6 {
        let ar = [
            a.to_be(),
            b.to_be(),
            c.to_be(),
            d.to_be(),
            e.to_be(),
            f.to_be(),
            g.to_be(),
            h.to_be(),
        ];
        IpAddrV6::from(unsafe { mem::transmute(ar) }, 0)
    }

    /// Returns a IP-v6 address with set a scope-id.
    ///
    /// The result will represent the IP address `a`:`b`:`c`:`d`:`e`:`f`:`g`:`h`%[scope-id]
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::with_scope_id(0,0,0,0,0,0,0,1,0x01);
    /// ```
    pub fn with_scope_id(
        a: u16,
        b: u16,
        c: u16,
        d: u16,
        e: u16,
        f: u16,
        g: u16,
        h: u16,
        scope_id: u32,
    ) -> IpAddrV6 {
        let ar = [
            a.to_be(),
            b.to_be(),
            c.to_be(),
            d.to_be(),
            e.to_be(),
            f.to_be(),
            g.to_be(),
            h.to_be(),
        ];
        IpAddrV6::from(unsafe { mem::transmute(ar) }, scope_id)
    }

    /// Returns a unspecified IP-v6 address.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::any();
    /// assert_eq!(ip, IpAddrV6::new(0,0,0,0,0,0,0,0));
    /// ```
    pub fn any() -> IpAddrV6 {
        IpAddrV6 {
            scope_id: 0,
            bytes: [0; 16],
        }
    }

    /// Returns a loopback IP-v6 address.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::loopback();
    /// assert_eq!(ip, IpAddrV6::new(0,0,0,0,0,0,0,1));
    /// ```
    pub fn loopback() -> IpAddrV6 {
        IpAddrV6 {
            scope_id: 0,
            bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        }
    }

    /// Returns a IP-v6 address from 16-octet bytes.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::from([0,1,2,3, 4,5,6,7, 8,9,10,11, 12,13,14,15], 0);
    /// assert_eq!(ip, IpAddrV6::new(0x0001, 0x0203,0x0405,0x0607,0x0809,0x0A0B, 0x0C0D, 0x0E0F));
    /// ```
    pub fn from(bytes: [u8; 16], scope_id: u32) -> IpAddrV6 {
        IpAddrV6 {
            scope_id: scope_id,
            bytes: bytes,
        }
    }

    /// Returns a scope-id.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::with_scope_id(0,0,0,0,0,0,0,0, 10);
    /// assert_eq!(ip.get_scope_id(), 10);
    /// ```
    pub fn get_scope_id(&self) -> u32 {
        self.scope_id
    }

    /// Sets a scope-id.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV6;
    ///
    /// let mut ip = IpAddrV6::loopback();
    /// assert_eq!(ip.get_scope_id(), 0);
    ///
    /// ip.set_scope_id(0x10);
    /// assert_eq!(ip.get_scope_id(), 16);
    /// ```
    pub fn set_scope_id(&mut self, scope_id: u32) {
        self.scope_id = scope_id
    }

    /// Returns true if this is a unspecified address.
    pub fn is_unspecified(&self) -> bool {
        self.bytes.iter().all(|&x| x == 0)
    }

    /// Returns true if this is a loopback address.
    pub fn is_loopback(&self) -> bool {
        (self.bytes[0] == 0 && self.bytes[1] == 0 && self.bytes[2] == 0 &&
             self.bytes[3] == 0 && self.bytes[4] == 0 && self.bytes[5] == 0 &&
             self.bytes[6] == 0 && self.bytes[7] == 0 && self.bytes[8] == 0 &&
             self.bytes[9] == 0 && self.bytes[10] == 0 &&
             self.bytes[11] == 0 && self.bytes[12] == 0 && self.bytes[13] == 0 &&
             self.bytes[14] == 0 && self.bytes[15] == 1)
    }

    /// Returns true if this is a link-local address.
    pub fn is_link_local(&self) -> bool {
        self.bytes[0] == 0xFE && (self.bytes[1] & 0xC0) == 0x80
    }

    /// Returns true if this is a site-local address.
    pub fn is_site_local(&self) -> bool {
        self.bytes[0] == 0xFE && (self.bytes[1] & 0xC0) == 0xC0
    }

    /// Returns true if this is a some multicast address.
    pub fn is_multicast(&self) -> bool {
        self.bytes[0] == 0xFF
    }

    /// Returns true if this is a multicast address for global.
    pub fn is_multicast_global(&self) -> bool {
        self.bytes[0] == 0xFF && (self.bytes[1] & 0x0F) == 0x0E
    }

    /// Returns true if this is a multicast address for link-local.
    pub fn is_multicast_link_local(&self) -> bool {
        self.bytes[0] == 0xFF && (self.bytes[1] & 0x0F) == 0x02
    }

    /// Returns true if this is a multicast address for node-local.
    pub fn is_multicast_node_local(&self) -> bool {
        self.bytes[0] == 0xFF && (self.bytes[1] & 0x0F) == 0x01
    }

    /// Returns true if this is a multicast address for org-local.
    pub fn is_multicast_org_local(&self) -> bool {
        self.bytes[0] == 0xFF && (self.bytes[1] & 0x0F) == 0x08
    }

    /// Returns true if this is a multicast address for site-local.
    pub fn is_multicast_site_local(&self) -> bool {
        self.bytes[0] == 0xFF && (self.bytes[1] & 0x0F) == 0x05
    }

    /// Returns true if this is a mapped IP-v4 address.
    pub fn is_v4_mapped(&self) -> bool {
        (self.bytes[0] == 0 && self.bytes[1] == 0 && self.bytes[2] == 0 &&
             self.bytes[3] == 0 && self.bytes[4] == 0 && self.bytes[5] == 0 &&
             self.bytes[6] == 0 && self.bytes[7] == 0 && self.bytes[8] == 0 &&
             self.bytes[9] == 0 && self.bytes[10] == 0xFF && self.bytes[11] == 0xFF)
    }

    /// Returns true if this is a IP-v4 compatible address.
    pub fn is_v4_compatible(&self) -> bool {
        ((self.bytes[0] == 0 && self.bytes[1] == 0 && self.bytes[2] == 0 &&
              self.bytes[3] == 0 && self.bytes[4] == 0 && self.bytes[5] == 0 &&
              self.bytes[6] == 0 && self.bytes[7] == 0 &&
              self.bytes[8] == 0 &&
              self.bytes[9] == 0 && self.bytes[10] == 0 &&
              self.bytes[11] == 0) &&
             !(self.bytes[12] == 0 && self.bytes[13] == 0 && self.bytes[14] == 0 &&
                   (self.bytes[15] == 0 || self.bytes[15] == 1)))
    }

    /// Retruns a 16 octets array.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }

    /// Retruns a IP-v4 address if this is a convertable address.
    pub fn to_v4(&self) -> Option<IpAddrV4> {
        if self.is_v4_mapped() || self.is_v4_compatible() {
            Some(IpAddrV4 {
                bytes: [
                    self.bytes[12],
                    self.bytes[13],
                    self.bytes[14],
                    self.bytes[15],
                ],
            })
        } else {
            None
        }
    }

    /// Returns a mapped IP-v4 address.
    ///
    /// Ex. 192.168.0.1 => ::ffff:192.168.0.1
    pub fn v4_mapped(addr: &IpAddrV4) -> Self {
        IpAddrV6 {
            scope_id: 0,
            bytes: [
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0xFF,
                0xFF,
                addr.bytes[0],
                addr.bytes[1],
                addr.bytes[2],
                addr.bytes[3],
            ],
        }
    }

    /// Returns a IP-v4 compatible address if the `addr` isn't in `0.0.0.0`, `0.0.0.1`.
    ///
    /// Ex. 192.168.0.1 => ::192.168.0.1
    pub fn v4_compatible(addr: &IpAddrV4) -> Option<Self> {
        if addr.bytes[0] == 0 && addr.bytes[1] == 0 && addr.bytes[2] == 0 &&
            (addr.bytes[3] == 0 || addr.bytes[3] == 1)
        {
            None
        } else {
            Some(IpAddrV6 {
                scope_id: 0,
                bytes: [
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    addr.bytes[0],
                    addr.bytes[1],
                    addr.bytes[2],
                    addr.bytes[3],
                ],
            })
        }
    }
}

impl AddAssign<i64> for IpAddrV6 {
    fn add_assign(&mut self, rhs: i64) {
        add_assign(&mut self.bytes, rhs)
    }
}

impl SubAssign<i64> for IpAddrV6 {
    fn sub_assign(&mut self, rhs: i64) {
        sub_assign(&mut self.bytes, rhs)
    }
}

impl fmt::Display for IpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_v6(&self.bytes, f)
    }
}

impl fmt::Debug for IpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<[u8; 16]> for IpAddrV6 {
    fn from(bytes: [u8; 16]) -> Self {
        IpAddrV6 {
            scope_id: 0,
            bytes: bytes,
        }
    }
}

impl From<net::Ipv6Addr> for IpAddrV6 {
    fn from(ip: net::Ipv6Addr) -> Self {
        ip.octets().into()
    }
}

/// Implements version-independent IP addresses.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum IpAddr {
    V4(IpAddrV4),
    V6(IpAddrV6),
}

impl IpAddr {
    /// Return true if this is unspecified address.
    pub fn is_unspecified(&self) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr.is_unspecified(),
            &IpAddr::V6(ref addr) => addr.is_unspecified(),
        }
    }

    /// Return true if this is loopback address.
    pub fn is_loopback(&self) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr.is_loopback(),
            &IpAddr::V6(ref addr) => addr.is_loopback(),
        }
    }

    /// Return true if this is multicast address.
    pub fn is_multicast(&self) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr.is_multicast(),
            &IpAddr::V6(ref addr) => addr.is_multicast(),
        }
    }

    /// Returns bytes of `&[u8; 4]` or `&[u8; 16]`.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            &IpAddr::V4(ref addr) => addr.as_bytes(),
            &IpAddr::V6(ref addr) => addr.as_bytes(),
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

impl fmt::Debug for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<net::IpAddr> for IpAddr {
    fn from(ip: net::IpAddr) -> Self {
        match ip {
            net::IpAddr::V4(addr) => IpAddr::V4(addr.octets().into()),
            net::IpAddr::V6(addr) => IpAddr::V6(addr.octets().into()),
        }
    }
}

/// Implements Prefix IP version 4 style addresses.
pub struct PrefixIpAddrV4 {
    bytes: [u8; 4],
    len: u8,
}

impl PrefixIpAddrV4 {
    fn masking(lhs: IpAddrV4, rhs: IpAddrV4) -> [u8; 4] {
        unsafe {
            let lhs: u32 = mem::transmute(lhs);
            let rhs: u32 = mem::transmute(rhs);
            mem::transmute(lhs & rhs)
        }
    }

    /// Returns new PrefixIpAddrV4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, PrefixIpAddrV4};
    ///
    /// assert!(PrefixIpAddrV4::new(IpAddrV4::new(192,168,100,1),
    ///                             IpAddrV4::new(255,255,255,0)).is_some());
    ///
    /// assert!(PrefixIpAddrV4::new(IpAddrV4::new(192,168,100,1),
    ///                             IpAddrV4::new(0,0,0,255)).is_none());
    /// ```
    pub fn new(addr: IpAddrV4, netmask: IpAddrV4) -> Option<PrefixIpAddrV4> {
        let len = netmask_len(&netmask.bytes);
        debug_assert!(len <= 32);
        if len != 0 {
            Some(PrefixIpAddrV4 {
                bytes: Self::masking(addr, netmask),
                len: len,
            })
        } else {
            None
        }
    }

    /// Returns new PrefixIpAddrV4.
    ///
    /// # Panics
    /// Panics if len == 0 or len > 32
    ///
    /// ```rust,no_run
    /// use asyncio::ip::{IpAddrV4, PrefixIpAddrV4};
    ///
    /// PrefixIpAddrV4::from(IpAddrV4::any(), 0);  // panic!
    /// ```
    pub fn from(addr: IpAddrV4, len: u8) -> PrefixIpAddrV4 {
        assert!(1 <= len && len <= 32);
        PrefixIpAddrV4 {
            bytes: Self::masking(addr, (u32::max_value() << (32 - len)).into()),
            len: len,
        }
    }

    /// Returns a network address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, PrefixIpAddrV4};
    ///
    /// let lo = PrefixIpAddrV4::from(IpAddrV4::loopback(), 8);
    /// assert_eq!(lo.network(), IpAddrV4::new(127,0,0,0));
    /// ```
    pub fn network(&self) -> IpAddrV4 {
        self.bytes.into()
    }

    /// Returns a subnet mask.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, PrefixIpAddrV4};
    ///
    /// let lo = PrefixIpAddrV4::from(IpAddrV4::loopback(), 8);
    /// assert_eq!(lo.netmask(), IpAddrV4::new(255,0,0,0));
    /// ```
    pub fn netmask(&self) -> IpAddrV4 {
        (u32::max_value() << (32 - self.len)).into()
    }

    /// Returns a length of subnet mask.
    pub fn netmask_len(&self) -> u8 {
        self.len
    }
}

impl fmt::Display for PrefixIpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}/{}",
            self.bytes[0],
            self.bytes[1],
            self.bytes[2],
            self.bytes[3],
            self.len
        )
    }
}

impl fmt::Debug for PrefixIpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// Implements Prefix IP version 6 style addresses.
pub struct PrefixIpAddrV6 {
    bytes: [u8; 16],
    len: u8,
}

impl PrefixIpAddrV6 {
    fn masking(lhs: [u8; 16], rhs: [u8; 16]) -> [u8; 16] {
        unsafe {
            let lhs: [u64; 2] = mem::transmute(lhs);
            let rhs: [u64; 2] = mem::transmute(rhs);
            mem::transmute([lhs[0] & rhs[0], lhs[1] & rhs[1]])
        }
    }

    fn make_netmask(len: u8) -> [u8; 16] {
        let bytes = match len.cmp(&64) {
            Ordering::Less => [(!((1u64 << (64 - len)) - 1)).to_be(), 0],
            Ordering::Equal => [u64::max_value(), 0],
            Ordering::Greater => [u64::max_value(), (!((1u64 << (128 - len)) - 1)).to_be()],
        };
        unsafe { mem::transmute(bytes) }
    }

    /// Returns new PrefixIpAddrV6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV6, PrefixIpAddrV6};
    ///
    /// assert!(PrefixIpAddrV6::new(IpAddrV6::loopback(),
    ///                             IpAddrV6::new(0xffff,0xffff,0xffff,0xffff,0,0,0,0)).is_some());
    ///
    /// assert!(PrefixIpAddrV6::new(IpAddrV6::loopback(),
    ///                             IpAddrV6::any()).is_none());
    /// ```
    pub fn new(addr: IpAddrV6, netmask: IpAddrV6) -> Option<PrefixIpAddrV6> {
        let len = netmask_len(&netmask.bytes);
        debug_assert!(len <= 128);
        if len != 0 {
            Some(PrefixIpAddrV6 {
                bytes: Self::masking(addr.bytes, netmask.bytes),
                len: len,
            })
        } else {
            None
        }
    }

    /// Returns new PrefixIpAddrV6.
    ///
    /// # Panics
    ///
    /// Panics if len == 0 or len > 128
    ///
    /// ```rust,no_run
    /// use asyncio::ip::{IpAddrV6, PrefixIpAddrV6};
    ///
    /// PrefixIpAddrV6::from(IpAddrV6::loopback(), 0);  // panic!
    /// ```
    pub fn from(addr: IpAddrV6, len: u8) -> PrefixIpAddrV6 {
        assert!(1 <= len && len <= 128);
        PrefixIpAddrV6 {
            bytes: Self::masking(addr.bytes, Self::make_netmask(len)),
            len: len,
        }
    }

    /// Returns a prefix address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV6, PrefixIpAddrV6};
    ///
    /// let lo = PrefixIpAddrV6::from(IpAddrV6::loopback(), 64);
    /// assert_eq!(lo.prefix(), IpAddrV6::any());
    /// ```
    pub fn prefix(&self) -> IpAddrV6 {
        self.bytes.into()
    }

    /// Returns a subnet mask.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV6, PrefixIpAddrV6};
    ///
    /// let lo = PrefixIpAddrV6::from(IpAddrV6::loopback(), 64);
    /// assert_eq!(lo.netmask(), IpAddrV6::new(0xffff,0xffff,0xffff,0xffff,0,0,0,0));
    /// ```
    pub fn netmask(&self) -> IpAddrV6 {
        Self::make_netmask(self.len).into()
    }

    /// Returns a length of subnet mask.
    pub fn netmask_len(&self) -> u8 {
        self.len
    }
}

impl fmt::Display for PrefixIpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(fmt_v6(&self.bytes, f));
        write!(f, "/{}", self.len)
    }
}

impl fmt::Debug for PrefixIpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

pub trait IpProtocol: Protocol + Eq + fmt::Display {
    fn async_connect<F>(soc: &Self::Socket, ep: &IpEndpoint<Self>, handler: F) -> F::Output
    where
        F: Handler<(), io::Error>;

    fn connect(soc: &Self::Socket, ep: &IpEndpoint<Self>) -> io::Result<()>;

    fn v4() -> Self;

    fn v6() -> Self;
}

/// The endpoint of internet protocol.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IpEndpoint<P> {
    ss: SockAddr<sockaddr_storage>,
    _marker: PhantomData<P>,
}

impl<P> IpEndpoint<P>
where
    P: IpProtocol,
{
    /// Returns a IpEndpoint from IP address and port number.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpEndpoint, IpAddrV4, Tcp};
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV4::loopback(), 80);
    /// ```
    pub fn new<T>(addr: T, port: u16) -> Self
    where
        T: IntoEndpoint<P>,
    {
        addr.into_endpoint(port)
    }

    /// Returns true if this is IpEndpoint of IP-v4 address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpEndpoint, IpAddrV4, IpAddrV6, Tcp};
    ///
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV4::loopback(), 80);
    /// assert_eq!(ep.is_v4(), true);
    ///
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV6::loopback(), 80);
    /// assert_eq!(ep.is_v4(), false);
    /// ```
    pub fn is_v4(&self) -> bool {
        self.ss.sa.ss_family as i32 == AF_INET
    }

    /// Returns true if this is IpEndpoint of IP-v6 address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpEndpoint, IpAddrV4, IpAddrV6, Tcp};
    ///
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV4::loopback(), 80);
    /// assert_eq!(ep.is_v6(), false);
    ///
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV6::loopback(), 80);
    /// assert_eq!(ep.is_v6(), true);
    /// ```
    pub fn is_v6(&self) -> bool {
        self.ss.sa.ss_family as i32 == AF_INET6
    }

    /// Returns a IP address.
    pub fn addr(&self) -> IpAddr {
        match self.ss.sa.ss_family as i32 {
            AF_INET => unsafe {
                let sin = &*(&self.ss.sa as *const _ as *const sockaddr_in);
                let bytes: [u8; 4] = mem::transmute(sin.sin_addr);
                IpAddr::V4(IpAddrV4::from(bytes))
            },
            AF_INET6 => unsafe {
                let sin6 = &*(&self.ss.sa as *const _ as *const sockaddr_in6);
                let bytes: [u8; 16] = mem::transmute(sin6.sin6_addr);
                IpAddr::V6(IpAddrV6::from(bytes, sin6.sin6_scope_id))
            },
            _ => unreachable!("Invalid address family ({}).", self.ss.sa.ss_family),
        }
    }

    /// Returns a port number.
    pub fn port(&self) -> u16 {
        let sin = unsafe { &*(&self.ss.sa as *const _ as *const sockaddr_in) };
        u16::from_be(sin.sin_port)
    }

    pub fn protocol(&self) -> P {
        if self.is_v4() {
            return P::v4();
        }
        if self.is_v6() {
            return P::v6();
        }
        unreachable!("Invalid address family ({}).", self.ss.sa.ss_family);
    }
}

impl<P> Endpoint<P> for IpEndpoint<P>
where
    P: IpProtocol,
{
    fn protocol(&self) -> P {
        let family_type = self.ss.sa.ss_family as i32;
        match family_type {
            AF_INET => P::v4(),
            AF_INET6 => P::v6(),
            _ => unreachable!("Invalid address family ({}).", family_type),
        }
    }

    fn as_ptr(&self) -> *const sockaddr {
        &self.ss.sa as *const _ as *const _
    }

    fn as_mut_ptr(&mut self) -> *mut sockaddr {
        &mut self.ss.sa as *mut _ as *mut _
    }

    fn capacity(&self) -> socklen_t {
        self.ss.capacity() as socklen_t
    }

    fn size(&self) -> socklen_t {
        self.ss.size() as socklen_t
    }

    unsafe fn resize(&mut self, len: socklen_t) {
        self.ss.resize(len as u8)
    }
}

impl<P: IpProtocol> fmt::Debug for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pro = self.protocol();
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "{}:{}:{}", pro, addr, self.port()),
            IpAddr::V6(addr) => write!(f, "{}:[{}]:{}", pro, addr, self.port()),
        }
    }
}

impl<P: IpProtocol> fmt::Display for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "{}:{}", addr, self.port()),
            IpAddr::V6(addr) => write!(f, "[{}]:{}", addr, self.port()),
        }
    }
}

impl<P: IpProtocol> From<(IpAddrV4, u16)> for IpEndpoint<P> {
    fn from(t: (IpAddrV4, u16)) -> Self {
        let mut ep = IpEndpoint {
            ss: SockAddr::new(AF_INET, mem::size_of::<sockaddr_in>() as u8),
            _marker: PhantomData,
        };
        unsafe {
            let sin = &mut *(&mut ep.ss.sa as *mut _ as *mut sockaddr_in);
            sin.sin_port = t.1.to_be();
            sin.sin_addr = mem::transmute(t.0);
            sin.sin_zero = [0; 8];
        }
        ep
    }
}

impl<P: IpProtocol> From<(IpAddrV6, u16)> for IpEndpoint<P> {
    fn from(t: (IpAddrV6, u16)) -> Self {
        let mut ep = IpEndpoint {
            ss: SockAddr::new(AF_INET6, mem::size_of::<sockaddr_in6>() as u8),
            _marker: PhantomData,
        };
        unsafe {
            let sin6 = &mut *(&mut ep.ss.sa as *mut _ as *mut sockaddr_in6);
            sin6.sin6_port = t.1.to_be();
            sin6.sin6_flowinfo = 0;
            sin6.sin6_scope_id = t.0.get_scope_id();
            sin6.sin6_addr = mem::transmute(t.0.bytes);
        }
        ep
    }
}

/// Provides conversion to a IP-endpoint.
pub trait IntoEndpoint<P> {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P>;
}

impl<P: IpProtocol> IntoEndpoint<P> for P {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        if &self == &P::v4() {
            return IpEndpoint::from((IpAddrV4::any(), port));
        }
        if &self == &P::v6() {
            return IpEndpoint::from((IpAddrV6::any(), port));
        }
        unreachable!("Invalid protocol");
    }
}

impl<P> IntoEndpoint<P> for IpAddrV4
where
    P: IpProtocol,
{
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from((self, port))
    }
}

impl<P> IntoEndpoint<P> for IpAddrV6
where
    P: IpProtocol,
{
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from((self, port))
    }
}

impl<P> IntoEndpoint<P> for IpAddr
where
    P: IpProtocol,
{
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        match self {
            IpAddr::V4(addr) => IpEndpoint::from((addr, port)),
            IpAddr::V6(addr) => IpEndpoint::from((addr, port)),
        }
    }
}

impl<'a, P> IntoEndpoint<P> for &'a IpAddrV4
where
    P: IpProtocol,
{
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from((self.clone(), port))
    }
}

impl<'a, P> IntoEndpoint<P> for &'a IpAddrV6
where
    P: IpProtocol,
{
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from((self.clone(), port))
    }
}

impl<'a, P> IntoEndpoint<P> for &'a IpAddr
where
    P: IpProtocol,
{
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        match self {
            &IpAddr::V4(ref addr) => IpEndpoint::from((addr.clone(), port)),
            &IpAddr::V6(ref addr) => IpEndpoint::from((addr.clone(), port)),
        }
    }
}

/// Get the current host name.
///
/// # Examples
///
/// ```
/// use asyncio::IoContext;
/// use asyncio::ip::host_name;
///
/// let ctx = &IoContext::new().unwrap();
/// println!("{}", host_name(ctx).unwrap());
/// ```
pub fn host_name(_: &IoContext) -> io::Result<String> {
    Ok(gethostname()?)
}

mod resolver;
pub use self::resolver::*;

mod icmp;
pub use self::icmp::*;

mod udp;
pub use self::udp::*;

mod tcp;
pub use self::tcp::*;

mod options;
pub use self::options::*;

#[test]
fn test_lladdr() {
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
fn test_lladdr_format() {
    assert_eq!(
        format!("{}", LlAddr::new(1, 2, 3, 4, 5, 6)),
        "01:02:03:04:05:06"
    );
    assert_eq!(
        format!("{}", LlAddr::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF)),
        "AA:BB:CC:DD:EE:FF"
    );
}

#[test]
fn test_ipaddr_v4() {
    assert_eq!(IpAddrV4::default().bytes, [0, 0, 0, 0]);
    assert_eq!(IpAddrV4::new(1, 2, 3, 4).bytes, [1, 2, 3, 4]);
    assert_eq!(IpAddrV4::new(1, 2, 3, 4), IpAddrV4::from([1, 2, 3, 4]));
    assert!(IpAddrV4::new(1, 2, 3, 4) < IpAddrV4::new(1, 2, 3, 5));
    assert!(IpAddrV4::new(1, 2, 3, 4) < IpAddrV4::new(1, 2, 4, 0));
    assert!(IpAddrV4::new(1, 2, 3, 4) < IpAddrV4::new(1, 3, 0, 0));
    assert!(IpAddrV4::new(1, 2, 3, 4) < IpAddrV4::new(2, 0, 0, 0));
}

#[test]
fn test_ipaddr_v4_format() {
    assert_eq!(format!("{}", IpAddrV4::any()), "0.0.0.0");
    assert_eq!(format!("{}", IpAddrV4::loopback()), "127.0.0.1");
}

#[test]
fn test_ipaddr_v4_add() {
    let mut a = IpAddrV4::new(192, 168, 0, 1);
    a += 1;
    assert_eq!(a, IpAddrV4::new(192, 168, 0, 2));
    a += 100;
    assert_eq!(a, IpAddrV4::new(192, 168, 0, 102));
    a += 256 * 10;
    assert_eq!(a, IpAddrV4::new(192, 168, 10, 102));
}

#[test]
fn test_ipaddr_v4_sub() {
    let mut a = IpAddrV4::new(192, 168, 0, 1);
    a -= 1;
    assert_eq!(a, IpAddrV4::new(192, 168, 0, 0));
    a -= 100;
    assert_eq!(a, IpAddrV4::new(192, 167, 255, 156));
    a -= 256 * 10;
    assert_eq!(a, IpAddrV4::new(192, 167, 245, 156));
}

#[test]
fn test_ipaddr_v6() {
    assert_eq!(
        IpAddrV6::default().bytes,
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        IpAddrV6::new(
            0x0102,
            0x0304,
            0x0506,
            0x0708,
            0x090a,
            0x0b0c,
            0x0d0e,
            0x0f10,
        ).bytes,
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    );
    assert_eq!(
        IpAddrV6::new(
            0x0102,
            0x0304,
            0x0506,
            0x0708,
            0x090a,
            0x0b0c,
            0x0d0e,
            0x0f10,
        ),
        IpAddrV6::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], 0)
    );
    assert_eq!(
        IpAddrV6::with_scope_id(0, 0, 0, 0, 0, 0, 0, 0, 100).get_scope_id(),
        100
    );
    assert!(
        IpAddrV6::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], 0) <
            IpAddrV6::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 17], 0)
    );
    assert!(
        IpAddrV6::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], 0) <
            IpAddrV6::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 00], 0)
    );
    assert!(
        IpAddrV6::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], 0) <
            IpAddrV6::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 00, 00], 0)
    );
}

#[test]
fn test_ipaddr_v6_format() {
    assert_eq!(format!("{}", IpAddrV6::any()), "::");
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

#[test]
fn test_add_assign() {
    let mut a = [0, 0];
    add_assign(&mut a, 0xFF);
    assert_eq!(&a, &[0, 0xFF]);
    add_assign(&mut a, 0x01);
    assert_eq!(&a, &[1, 0]);
    add_assign(&mut a, 0x101);
    assert_eq!(&a, &[2, 1]);
}

#[test]
#[should_panic]
fn test_add_assign_overflow() {
    let mut a = [0xFF, 0xFF];
    add_assign(&mut a, 1);
}

#[test]
fn test_sub_assign() {
    let mut a = [0xFF, 0xFF];
    sub_assign(&mut a, 0xFF);
    assert_eq!(&a, &[0xFF, 0]);
    sub_assign(&mut a, 0x01);
    assert_eq!(&a, &[0xFE, 0xFF]);
    sub_assign(&mut a, 0x101);
    assert_eq!(&a, &[0xFD, 0xFE]);
}

#[test]
#[should_panic]
fn test_sub_assign_underflow() {
    let mut a = [0, 0];
    sub_assign(&mut a, 1);
}

#[test]
fn test_ipaddr_as_bytes() {
    let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let v4 = IpAddr::V4(IpAddrV4::new(1, 2, 3, 4));
    assert!(v4.as_bytes() == &bytes[..4]);
    assert!(v4.as_bytes() != &bytes[..5]);
    assert!(v4.as_bytes() != &bytes[1..5]);

    let v6 = IpAddr::V6(IpAddrV6::from(bytes.clone(), 0));
    assert!(v6.as_bytes() == &bytes[..]);
    assert!(v6.as_bytes() != v4.as_bytes());
}

#[test]
fn test_netmask_len() {
    assert_eq!(netmask_len(&[255, 255, 255, 0]), 24);
    assert_eq!(netmask_len(&[255, 255, 255, 255]), 32);
    assert_eq!(netmask_len(&[255, 255, 254, 0]), 23);
    assert_eq!(netmask_len(&[255, 255, 255, 254]), 31);
    assert_eq!(netmask_len(&[128, 0, 0, 0]), 1);

    assert_eq!(netmask_len(&[0, 0, 0, 0]), 0);
    assert_eq!(netmask_len(&[1, 1, 1, 1]), 0);
    assert_eq!(netmask_len(&[128, 1, 1, 1]), 0);
}

#[test]
fn test_prefix_ipaddr_v4() {
    let ip = PrefixIpAddrV4::new(
        IpAddrV4::new(192, 168, 0, 1),
        IpAddrV4::new(255, 255, 255, 0),
    ).unwrap();
    assert_eq!(ip.network(), IpAddrV4::new(192, 168, 0, 0));
    assert_eq!(ip.netmask(), IpAddrV4::new(255, 255, 255, 0));
    assert_eq!(ip.netmask_len(), 24);

    let ip = PrefixIpAddrV4::new(
        IpAddrV4::new(192, 168, 255, 1),
        IpAddrV4::new(255, 255, 240, 0),
    ).unwrap();
    assert_eq!(ip.network(), IpAddrV4::new(192, 168, 240, 0));
    assert_eq!(ip.netmask(), IpAddrV4::new(255, 255, 240, 0));
    assert_eq!(ip.netmask_len(), 20);
}

#[test]
#[should_panic]
fn test_prefix_ipaddr_v4_from_panic() {
    PrefixIpAddrV4::from(IpAddrV4::loopback(), 0);
}

#[test]
fn test_prefix_ipaddr_v4_format() {
    let ip = PrefixIpAddrV4::new(
        IpAddrV4::new(192, 168, 0, 1),
        IpAddrV4::new(255, 255, 255, 0),
    ).unwrap();
    assert_eq!(format!("{}", ip), "192.168.0.0/24");
}

#[test]
fn test_prefix_ipaddr_v6_half() {
    let netmask = IpAddrV6::new(0xffff, 0xffff, 0xffff, 0xffff, 0, 0, 0, 0);
    let ip = PrefixIpAddrV6::new(
        IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0xdead, 0xbeaf),
        netmask.clone(),
    ).unwrap();
    assert_eq!(ip.prefix(), IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0, 0));
    assert_eq!(ip.netmask(), netmask);
    assert_eq!(ip.netmask_len(), 64);
}

#[test]
fn test_prefix_ipaddr_v6_long() {
    let netmask = IpAddrV6::new(
        0xffff,
        0xffff,
        0xffff,
        0xffff,
        0xffff,
        0xffff,
        0xffff,
        0xfff0,
    );
    let ip = PrefixIpAddrV6::new(
        IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0xdead, 0xbeaf),
        netmask.clone(),
    ).unwrap();
    assert_eq!(
        ip.prefix(),
        IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0xdead, 0xbea0)
    );
    assert_eq!(ip.netmask(), netmask);
    assert_eq!(ip.netmask_len(), 124);
}

#[test]
fn test_prefix_ipaddr_v6_short() {
    let netmask = IpAddrV6::new(0xfe00, 0, 0, 0, 0, 0, 0, 0);
    let ip = PrefixIpAddrV6::new(
        IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0xdead, 0xbeaf),
        netmask.clone(),
    ).unwrap();
    assert_eq!(ip.prefix(), IpAddrV6::new(0x2000, 0, 0, 0, 0, 0, 0, 0));
    assert_eq!(ip.netmask(), netmask);
    assert_eq!(ip.netmask_len(), 7);
}

#[test]
#[should_panic]
fn test_prefix_ipaddr_v6_from_panic() {
    PrefixIpAddrV6::from(IpAddrV6::loopback(), 0);
}

#[test]
fn test_prefix_ipaddr_v6_format() {
    let ip = PrefixIpAddrV6::from(IpAddrV6::loopback(), 64);
    assert_eq!(format!("{}", ip), "::/64");

    let ip = PrefixIpAddrV6::from(IpAddrV6::new(0xdead, 0xbeaf, 0, 0, 0, 0, 0, 0), 32);
    assert_eq!(format!("{}", ip), "dead:beaf::/32");
}

#[test]
fn test_host_name() {
    let ctx = &IoContext::new().unwrap();
    host_name(ctx).unwrap();
}

#[test]
fn test_endpoint_v4() {
    let ep = UdpEndpoint::new(IpAddrV4::new(1, 2, 3, 4), 10);
    assert!(ep.is_v4());
    assert!(!ep.is_v6());
    assert_eq!(ep.addr(), IpAddr::V4(IpAddrV4::new(1, 2, 3, 4)));
    assert_eq!(ep.port(), 10);
}

#[test]
fn test_endpoint_v6() {
    let ep = TcpEndpoint::new(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8), 10);
    assert!(ep.is_v6());
    assert!(!ep.is_v4());
    assert_eq!(ep.addr(), IpAddr::V6(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8)));
    assert_eq!(ep.port(), 10);
}

#[test]
fn test_endpoint_cmp() {
    let a = IcmpEndpoint::new(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8), 10);
    let b = IcmpEndpoint::new(IpAddrV6::with_scope_id(1, 2, 3, 4, 5, 6, 7, 8, 1), 10);
    let c = IcmpEndpoint::new(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8), 11);
    assert!(a == a && b == b && c == c);
    assert!(a != b && b != c);
    assert!(a < b);
    assert!(b < c);
}
