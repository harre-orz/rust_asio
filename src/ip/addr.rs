use std::fmt;
use std::mem;
use std::ops::{AddAssign, SubAssign};

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
        LlAddr { bytes: [a,b,c,d,e,f] }
    }

    /// Returns from a 6-octet bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use asyncio::ip::LlAddr;
    ///
    /// let mac = LlAddr::from_bytes([1,2,3,4,5,6]);
    /// assert_eq!(mac, LlAddr::new(1,2,3,4,5,6));
    /// ```
    pub fn from_bytes(bytes: [u8; 6]) -> LlAddr {
        LlAddr { bytes: bytes }
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
        write!(f, "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
               self.bytes[0], self.bytes[1], self.bytes[2],
               self.bytes[3], self.bytes[4], self.bytes[5])
    }
}

impl fmt::Debug for LlAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

// fn is_netmask_impl(addr: &[u8]) -> bool {
//     if addr[0] == 0 {
//         return false;
//     }

//     let mut it = addr.iter();
//     while let Some(n) = it.next() {
//         match *n {
//             0b00000000 |
//             0b10000000 |
//             0b11000000 |
//             0b11100000 |
//             0b11110000 |
//             0b11111000 |
//             0b11111100 |
//             0b11111110 =>
//                 return it.all(|&x| x == 0),
//             0b11111111 => {},
//             _ => return false,
//         }
//     }
//     true
// }

// fn netmask_len_impl(addr: &[u8]) -> Option<u8> {
//     let mut len = 0;
//     let mut it = addr.iter();
//     while let Some(n) = it.next() {
//         if *n == 0b11111111 {
//             len += 8;
//         } else {
//             match *n {
//                 0b00000000 => len += 0,
//                 0b10000000 => len += 1,
//                 0b11000000 => len += 2,
//                 0b11100000 => len += 3,
//                 0b11110000 => len += 4,
//                 0b11111000 => len += 5,
//                 0b11111100 => len += 6,
//                 0b11111110 => len += 7,
//                 _ => return None,
//             }
//             return if it.all(|&x| x == 0) {
//                 Some(len)
//             } else {
//                 None
//             }
//         }
//     }
//     Some(len)
// }

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
        IpAddrV4 { bytes: [a,b,c,d] }
    }

    /// Returns from 4-octet bytes.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::from_bytes([172,16,0,1]);
    /// assert_eq!(ip, IpAddrV4::new(172,16,0,1));
    /// ```
    pub fn from_bytes(bytes: [u8; 4]) -> IpAddrV4 {
        IpAddrV4 { bytes: bytes }
    }

    /// Returns from integer in host byte order.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::from_u32(0x7F000001);
    /// assert_eq!(ip, IpAddrV4::new(127,0,0,1));
    /// ```
    pub fn from_u32(mut addr: u32) -> IpAddrV4 {
        let d = (addr & 0xFF) as u8;
        addr >>= 8;
        let c = (addr & 0xFF) as u8;
        addr >>= 8;
        let b = (addr & 0xFF) as u8;
        addr >>= 8;
        IpAddrV4::new(addr as u8, b, c, d)
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
        IpAddrV4::new(127,0,0,1)
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

    // /// Returns true for if this is a subnet netmask.
    // ///
    // /// # Examples.
    // /// ```
    // /// use asyncio::ip::IpAddrV4;
    // ///
    // /// assert!(IpAddrV4::new(255,255,255,0).is_netmask());
    // /// ```
    // pub fn is_netmask(&self) -> bool {
    //     is_netmask_impl(&self.addr)
    // }

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
        ((((((self.bytes[0] as u32) << 8)
            + self.bytes[1] as u32) << 8)
            + self.bytes[2] as u32) << 8)
            + self.bytes[3] as u32
    }

    // /// Returns length of subnet mask if this is a subnet mask.
    // ///
    // /// # Examples
    // /// ```
    // /// use asyncio::ip::IpAddrV4;
    // ///
    // /// assert_eq!(IpAddrV4::new(255,255,0,0).netmask_len().unwrap(), 16);
    // /// assert!(IpAddrV4::new(255,255,0,1).netmask_len().is_none());
    // /// ```
    // pub fn netmask_len(&self) -> Option<u8> {
    //     netmask_len_impl(&self.addr)
    // }
}

impl AddAssign<i64> for IpAddrV4 {
    fn add_assign(&mut self, rhs: i64) {
        *self = Self::from_u32(self.to_u32() + rhs as u32);
    }
}

impl SubAssign<i64> for IpAddrV4 {
    fn sub_assign(&mut self, rhs: i64) {
        *self = Self::from_u32(self.to_u32() - rhs as u32);
    }
}

impl fmt::Display for IpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}",
               self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3])
    }
}

impl fmt::Debug for IpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
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
        let ar = [ a.to_be(), b.to_be(), c.to_be(), d.to_be(), e.to_be(), f.to_be(), g.to_be(), h.to_be() ];
        IpAddrV6::from_bytes(unsafe { mem::transmute(ar) }, 0)
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
    pub fn with_scope_id(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16, scope_id: u32) -> IpAddrV6 {
        let ar = [ a.to_be(), b.to_be(), c.to_be(), d.to_be(), e.to_be(), f.to_be(), g.to_be(), h.to_be() ];
        IpAddrV6::from_bytes(unsafe { mem::transmute(ar) }, scope_id)
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
        IpAddrV6 { scope_id: 0, bytes: [0; 16] }
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
        IpAddrV6 { scope_id: 0, bytes: [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1] }
    }

    /// Returns a IP-v6 address from 16-octet bytes.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::from_bytes([0,1,2,3, 4,5,6,7, 8,9,10,11, 12,13,14,15], 0);
    /// assert_eq!(ip, IpAddrV6::new(0x0001, 0x0203,0x0405,0x0607,0x0809,0x0A0B, 0x0C0D, 0x0E0F));
    /// ```
    pub fn from_bytes(bytes: [u8; 16], scope_id: u32) -> IpAddrV6 {
        IpAddrV6 { scope_id: scope_id, bytes: bytes }
    }

    /// Returns a scope-id.
    ///
    /// # Examples
    /// ```
    /// use asyncio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::with_scope_id(0,0,0,0,0,0,0,0,0x10);
    /// assert_eq!(ip.get_scope_id(), 16);
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
        (self.bytes[0] == 0 && self.bytes[1] == 0 && self.bytes[2] == 0 && self.bytes[3] == 0 &&
         self.bytes[4] == 0 && self.bytes[5] == 0 && self.bytes[6] == 0 && self.bytes[7] == 0 &&
         self.bytes[8] == 0 && self.bytes[9] == 0 && self.bytes[10] == 0 && self.bytes[11] == 0 &&
         self.bytes[12] == 0 && self.bytes[13] == 0 && self.bytes[14] == 0 && self.bytes[15] == 1)
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
        (self.bytes[0] == 0 && self.bytes[1] == 0 && self.bytes[2] == 0 && self.bytes[3] == 0 &&
         self.bytes[4] == 0 && self.bytes[5] == 0 && self.bytes[6] == 0 && self.bytes[7] == 0 &&
         self.bytes[8] == 0 && self.bytes[9] == 0 && self.bytes[10] == 0xFF && self.bytes[11] == 0xFF)
    }

    /// Returns true if this is a IP-v4 compatible address.
    pub fn is_v4_compatible(&self) -> bool {
        ((self.bytes[0] == 0 && self.bytes[1] == 0 && self.bytes[2] == 0 && self.bytes[3] == 0 &&
          self.bytes[4] == 0 && self.bytes[5] == 0 && self.bytes[6] == 0 && self.bytes[7] == 0 &&
          self.bytes[8] == 0 && self.bytes[9] == 0 && self.bytes[10] == 0 && self.bytes[11] == 0)
         && !(self.bytes[12] == 0 && self.bytes[13] == 0 && self.bytes[14] == 0
              && (self.bytes[15] == 0 || self.bytes[15] == 1)))
    }

    /// Retruns a 16 octets array.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }

    /// Retruns a IP-v4 address if this is a convertable address.
    pub fn to_v4(&self) -> Option<IpAddrV4> {
        if self.is_v4_mapped() || self.is_v4_compatible() {
            Some(IpAddrV4 { bytes: [ self.bytes[12], self.bytes[13], self.bytes[14], self.bytes[15] ] })
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
            bytes: [0,0,0,0,0,0,0,0,0,0,0xFF,0xFF,
                    addr.bytes[0], addr.bytes[1], addr.bytes[2], addr.bytes[3]]
        }
    }

    /// Returns a IP-v4 compatible address if the `addr` isn't in `0.0.0.0`, `0.0.0.1`.
    ///
    /// Ex. 192.168.0.1 => ::192.168.0.1
    pub fn v4_compatible(addr: &IpAddrV4) -> Option<Self> {
        if addr.bytes[0] == 0 && addr.bytes[1] == 0 && addr.bytes[2] == 0
            && (addr.bytes[3] == 0 || addr.bytes[3] == 1)
        {
            None
        } else {
            Some(IpAddrV6 {
                scope_id: 0,
                bytes: [0,0,0,0,0,0,0,0,0,0,0,0,
                        addr.bytes[0], addr.bytes[1], addr.bytes[2], addr.bytes[3]]
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
            return write!(f, "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                          u16::from_be(ar[0]), u16::from_be(ar[1]), u16::from_be(ar[2]), u16::from_be(ar[3]),
                          u16::from_be(ar[4]), u16::from_be(ar[5]), u16::from_be(ar[6]), u16::from_be(ar[7]));
        }

        if max_idx == 0 {
            try!(write!(f, ":"));
        } else {
            for i in 0..max_idx {
                try!(write!(f, "{:x}:", u16::from_be(ar[i])));
            }
        }

        if max_idx + max_cnt == 8 {
            try!(write!(f, ":"));
        } else {
            for i in max_idx + max_cnt..ar.len() {
                try!(write!(f, ":{:x}", u16::from_be(ar[i])));
            }
        }
        Ok(())
    }
}

impl fmt::Debug for IpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// implements version-independent IP addresses.
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

#[test]
fn test_lladdr() {
    assert_eq!(LlAddr::default().bytes, [0,0,0,0,0,0]);
    assert_eq!(LlAddr::new(1,2,3,4,5,6).bytes, [1,2,3,4,5,6]);
    assert!(LlAddr::new(1,2,3,4,5,6) == LlAddr::from_bytes([1,2,3,4,5,6]));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,3,4,5,7));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,3,4,6,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,3,5,0,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,4,0,0,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,3,0,0,0,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(2,0,0,0,0,0));
}

#[test]
fn test_lladdr_format() {
    assert_eq!(format!("{}", LlAddr::new(1,2,3,4,5,6)), "01:02:03:04:05:06");
    assert_eq!(format!("{}", LlAddr::new(0xAA,0xBB,0xCC,0xDD,0xEE,0xFF)), "AA:BB:CC:DD:EE:FF");
}

#[test]
fn test_ipaddr_v4() {
    assert_eq!(IpAddrV4::default().bytes, [0,0,0,0]);
    assert_eq!(IpAddrV4::new(1,2,3,4).bytes, [1,2,3,4]);
    assert_eq!(IpAddrV4::new(1,2,3,4), IpAddrV4::from_bytes([1,2,3,4]));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,2,3,5));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,2,4,0));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,3,0,0));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(2,0,0,0));
}

#[test]
fn test_ipaddr_v4_format() {
    assert_eq!(format!("{}", IpAddrV4::any()), "0.0.0.0");
    assert_eq!(format!("{}", IpAddrV4::loopback()), "127.0.0.1");
}

#[test]
fn test_ipaddr_v4_add() {
    let mut a = IpAddrV4::new(192,168,0,1);
    a += 1;
    assert_eq!(a, IpAddrV4::new(192,168,0,2));
    a += 100;
    assert_eq!(a, IpAddrV4::new(192,168,0,102));
    a += 256*10;
    assert_eq!(a, IpAddrV4::new(192,168,10,102));
}

#[test]
fn test_ipaddr_v4_sub() {
    let mut a = IpAddrV4::new(192,168,0,1);
    a -= 1;
    assert_eq!(a, IpAddrV4::new(192,168,0,0));
    a -= 100;
    assert_eq!(a, IpAddrV4::new(192,167,255,156));
    a -= 256*10;
    assert_eq!(a, IpAddrV4::new(192,167,245,156));
}

#[test]
fn test_ipaddr_v6() {
    assert_eq!(IpAddrV6::default().bytes, [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);
    assert_eq!(IpAddrV6::new(0x0102,0x0304,0x0506,0x0708,0x090a,0x0b0c,0x0d0e,0x0f10).bytes,
               [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
    assert_eq!(IpAddrV6::new(0x0102,0x0304,0x0506,0x0708,0x090a,0x0b0c,0x0d0e,0x0f10),
               IpAddrV6::from_bytes([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0));
    assert_eq!(IpAddrV6::with_scope_id(0,0,0,0,0,0,0,0,100).get_scope_id(), 100);
    assert!(IpAddrV6::from_bytes([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0) <
            IpAddrV6::from_bytes([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,17], 0));
    assert!(IpAddrV6::from_bytes([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0) <
            IpAddrV6::from_bytes([1,2,3,4,5,6,7,8,9,10,11,12,13,14,16,00], 0));
    assert!(IpAddrV6::from_bytes([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0) <
            IpAddrV6::from_bytes([1,2,3,4,5,6,7,8,9,10,11,12,13,15,00,00], 0));
}

#[test]
fn test_ipaddr_v6_format() {
    assert_eq!(format!("{}", IpAddrV6::any()), "::");
    assert_eq!(format!("{}", IpAddrV6::loopback()), "::1");
    assert_eq!(format!("{}", IpAddrV6::new(1,2,3,4,5,6,7,8)), "1:2:3:4:5:6:7:8");
    assert_eq!(format!("{}", IpAddrV6::new(0,2,3,4,5,6,7,8)), "::2:3:4:5:6:7:8");
    assert_eq!(format!("{}", IpAddrV6::new(1,2,3,4,5,6,7,0)), "1:2:3:4:5:6:7::");
    assert_eq!(format!("{}", IpAddrV6::new(1,2,3,4,0,6,7,8)), "1:2:3:4::6:7:8");
    assert_eq!(format!("{}", IpAddrV6::new(1,0,0,0,0,0,0,8)), "1::8");
}

#[test]
fn test_add_assign() {
    let mut a = [0,0];
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
