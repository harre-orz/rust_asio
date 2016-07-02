use std::fmt;
use std::mem;
use std::ops::{AddAssign, SubAssign};

/// Implements Link-layer addresses.
///
/// Also referred to as MAC address and Hardware address.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LlAddr {
    addr: [u8; 6],
}

impl LlAddr {
    /// Constructs a Link-layer address.
    ///
    /// The result will represent the LL-address a:b:c:d:e:f.
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> LlAddr {
        Self::from_bytes(&[a,b,c,d,e,f])
    }

    /// Constructs from a 6-octet bytes.
    fn from_bytes(addr: &[u8; 6]) -> LlAddr {
        LlAddr { addr: *addr }
    }
}

impl fmt::Display for LlAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:2x}:{:2x}:{:2x}:{:2x}:{:2x}:{:2x}",
               self.addr[0], self.addr[1], self.addr[2],
               self.addr[3], self.addr[4], self.addr[5])
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
    addr: [u8; 4],
}

impl IpAddrV4 {
    /// Constructs a IP-v4 address.
    ///
    /// The result will represent the IP address `a`.`b`.`c`.`d`.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::new(192,168,0,1);
    /// ```
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> IpAddrV4 {
        IpAddrV4 { addr: [a,b,c,d] }
    }

    /// Constructs from 4-octet bytes.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::from_bytes(&[172,16,0,1]);
    /// assert_eq!(ip, IpAddrV4::new(172,16,0,1));
    /// ```
    pub fn from_bytes(addr: &[u8; 4]) -> IpAddrV4 {
        IpAddrV4 { addr: *addr }
    }

    /// Constructs from integer in host byte order.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::from_ulong(0x7F000001);
    /// assert_eq!(ip, IpAddrV4::new(127,0,0,1));
    /// ```
    pub fn from_ulong(mut addr: u32) -> IpAddrV4 {
        let d = (addr & 0xFF) as u8;
        addr >>= 8;
        let c = (addr & 0xFF) as u8;
        addr >>= 8;
        let b = (addr & 0xFF) as u8;
        addr >>= 8;
        IpAddrV4::new(addr as u8, b, c, d)
    }

    /// Constructs a unspecified IP-v4 address.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// let ip = IpAddrV4::any();
    /// assert_eq!(ip, IpAddrV4::new(0,0,0,0));
    /// ```
    pub fn any() -> IpAddrV4 {
        IpAddrV4 { addr: [0; 4] }
    }

    /// Constructs a IP-v4 address for a loopback address.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
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
    /// use asio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::any().is_unspecified());
    /// ```
    pub fn is_unspecified(&self) -> bool {
        self.addr.iter().all(|&x| x == 0)
    }

    /// Return true for if this is a loopback address 127.0.0.1.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::loopback().is_loopback());
    /// ```
    pub fn is_loopback(&self) -> bool {
        (self.addr[0] & 0xFF) == 0x7F
    }

    /// Returns true for if this is a class A address.
    ///
    /// The class A address ranges:
    ///
    /// - 10.0.0.0/8
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(10,0,0,1).is_class_a());
    /// ```
    pub fn is_class_a(&self) -> bool {
        (self.addr[0] & 0x80) == 0
    }

    /// Returns true for if this is a class B address.
    ///
    /// The class B address ranges:
    ///
    /// - 172.16.0.0/12
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(172,16,0,1).is_class_b());
    /// ```
    pub fn is_class_b(&self) -> bool {
        (self.addr[0] & 0xC0) == 0x80
    }

    /// Returns true for if this is a class C address.
    ///
    /// The class c address ranges:
    ///
    /// - 192.168.0.0/16
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(192,168,0,1).is_class_c());
    /// ```
    pub fn is_class_c(&self) -> bool {
        (self.addr[0] & 0xE0) == 0xC0
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
    /// use asio::ip::IpAddrV4;
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
    /// use asio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(224,0,0,1).is_multicast());
    /// ```
    pub fn is_multicast(&self) -> bool {
        (self.addr[0] & 0xF0) == 0xE0
    }

    /// Returns true for if this is a link-local address.
    ///
    /// The link-local address ranges:
    ///
    /// - 169.254.0.0/16
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// assert!(IpAddrV4::new(169,254,0,0).is_link_local());
    /// ```
    pub fn is_link_local(&self) -> bool {
        self.addr[0] == 0xA9 && self.addr[1] == 0xFE
    }

    // /// Returns true for if this is a subnet netmask.
    // ///
    // /// # Examples.
    // /// ```
    // /// use asio::ip::IpAddrV4;
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
    /// use asio::ip::IpAddrV4;
    ///
    /// assert_eq!(IpAddrV4::new(169,254,0,1).as_bytes(), &[169,254,0,1]);
    /// ```
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.addr
    }

    /// Returns `u32` in host byte order.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV4;
    ///
    /// assert_eq!(IpAddrV4::new(10,0,0,1).to_ulong(), 10*256*256*256+1);
    /// ```
    pub fn to_ulong(&self) -> u32 {
        ((((((self.addr[0] as u32) << 8)
            + self.addr[1] as u32) << 8)
            + self.addr[2] as u32) << 8)
            + self.addr[3] as u32
    }

    // /// Returns length of subnet mask if this is a subnet mask.
    // ///
    // /// # Examples
    // /// ```
    // /// use asio::ip::IpAddrV4;
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
        *self = Self::from_ulong(self.to_ulong() + rhs as u32);
    }
}

impl SubAssign<i64> for IpAddrV4 {
    fn sub_assign(&mut self, rhs: i64) {
        *self = Self::from_ulong(self.to_ulong() - rhs as u32);
    }
}

impl fmt::Display for IpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}",
               self.addr[0], self.addr[1], self.addr[2], self.addr[3])
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
    addr: [u8; 16],
}

impl IpAddrV6 {
    /// Constructs a IP-v6 address.
    ///
    /// The result will represent the IP address `a`:`b`:`c`:`d`:`e`:`f`:`g`:`h`
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::new(0,0,0,0,0,0,0,1);
    /// ```
    pub fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> IpAddrV6 {
        let ar = [ a.to_be(), b.to_be(), c.to_be(), d.to_be(), e.to_be(), f.to_be(), g.to_be(), h.to_be() ];
        IpAddrV6::from_bytes(unsafe { mem::transmute(&ar) }, 0)
    }

    /// Constructs a IP-v6 address with set a scope-id.
    ///
    /// The result will represent the IP address `a`:`b`:`c`:`d`:`e`:`f`:`g`:`h`%[scope-id]
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::with_scope_id(0,0,0,0,0,0,0,1,0x01);
    /// ```
    pub fn with_scope_id(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16, scope_id: u32) -> IpAddrV6 {
        let ar = [ a.to_be(), b.to_be(), c.to_be(), d.to_be(), e.to_be(), f.to_be(), g.to_be(), h.to_be() ];
        IpAddrV6::from_bytes(unsafe { mem::transmute(&ar) }, scope_id)
    }

    /// Constructs a unspecified IP-v6 address.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::any();
    /// assert_eq!(ip, IpAddrV6::new(0,0,0,0,0,0,0,0));
    /// ```
    pub fn any() -> IpAddrV6 {
        IpAddrV6 { scope_id: 0, addr: [0; 16] }
    }

    /// Constructs a loopback IP-v6 address.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::loopback();
    /// assert_eq!(ip, IpAddrV6::new(0,0,0,0,0,0,0,1));
    /// ```
    pub fn loopback() -> IpAddrV6 {
        IpAddrV6 { scope_id: 0, addr: [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1] }
    }

    /// Constructs a IP-v6 address from 16-octet bytes.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV6;
    ///
    /// let ip = IpAddrV6::from_bytes(&[0,1,2,3, 4,5,6,7, 8,9,10,11, 12,13,14,15], 0);
    /// assert_eq!(ip, IpAddrV6::new(0x0001, 0x0203,0x0405,0x0607,0x0809,0x0A0B, 0x0C0D, 0x0E0F));
    /// ```
    pub fn from_bytes(addr: &[u8; 16], scope_id: u32) -> IpAddrV6 {
        IpAddrV6 { scope_id: scope_id, addr: *addr }
    }

    /// Returns a scope-id.
    ///
    /// # Examples
    /// ```
    /// use asio::ip::IpAddrV6;
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
    /// use asio::ip::IpAddrV6;
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
        self.addr.iter().all(|&x| x == 0)
    }

    /// Returns true if this is a loopback address.
    pub fn is_loopback(&self) -> bool {
        (self.addr[0] == 0 && self.addr[1] == 0 && self.addr[2] == 0 && self.addr[3] == 0 &&
         self.addr[4] == 0 && self.addr[5] == 0 && self.addr[6] == 0 && self.addr[7] == 0 &&
         self.addr[8] == 0 && self.addr[9] == 0 && self.addr[10] == 0 && self.addr[11] == 0 &&
         self.addr[12] == 0 && self.addr[13] == 0 && self.addr[14] == 0 && self.addr[15] == 1)
    }

    /// Returns true if this is a link-local address.
    pub fn is_link_local(&self) -> bool {
        self.addr[0] == 0xFE && (self.addr[1] & 0xC0) == 0x80
    }

    /// Returns true if this is a site-local address.
    pub fn is_site_local(&self) -> bool {
        self.addr[0] == 0xFE && (self.addr[1] & 0xC0) == 0xC0
    }

    /// Returns true if this is a some multicast address.
    pub fn is_multicast(&self) -> bool {
        self.addr[0] == 0xFF
    }

    /// Returns true if this is a multicast address for global.
    pub fn is_multicast_global(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x0E
    }

    /// Returns true if this is a multicast address for link-local.
    pub fn is_multicast_link_local(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x02
    }

    /// Returns true if this is a multicast address for node-local.
    pub fn is_multicast_node_local(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x01
    }

    /// Returns true if this is a multicast address for org-local.
    pub fn is_multicast_org_local(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x08
    }

    /// Returns true if this is a multicast address for site-local.
    pub fn is_multicast_site_local(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x05
    }

    /// Returns true if this is a mapped IP-v4 address.
    pub fn is_v4_mapped(&self) -> bool {
        (self.addr[0] == 0 && self.addr[1] == 0 && self.addr[2] == 0 && self.addr[3] == 0 &&
         self.addr[4] == 0 && self.addr[5] == 0 && self.addr[6] == 0 && self.addr[7] == 0 &&
         self.addr[8] == 0 && self.addr[9] == 0 && self.addr[10] == 0xFF && self.addr[11] == 0xFF)
    }

    /// Returns true if this is a IP-v4 compatible address.
    pub fn is_v4_compatible(&self) -> bool {
        ((self.addr[0] == 0 && self.addr[1] == 0 && self.addr[2] == 0 && self.addr[3] == 0 &&
          self.addr[4] == 0 && self.addr[5] == 0 && self.addr[6] == 0 && self.addr[7] == 0 &&
          self.addr[8] == 0 && self.addr[9] == 0 && self.addr[10] == 0 && self.addr[11] == 0)
         && !(self.addr[12] == 0 && self.addr[13] == 0 && self.addr[14] == 0
              && (self.addr[15] == 0 || self.addr[15] == 1)))
    }

    /// Retruns a 16 octets array.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.addr
    }

    /// Retruns a IP-v4 address if this is a convertable address.
    pub fn to_v4(&self) -> Option<IpAddrV4> {
        if self.is_v4_mapped() || self.is_v4_compatible() {
            Some(IpAddrV4 { addr: [ self.addr[12], self.addr[13], self.addr[14], self.addr[15] ] })
        } else {
            None
        }
    }

    /// Constructs a mapped IP-v4 address.
    ///
    /// Ex. 192.168.0.1 => ::ffff:192.168.0.1
    pub fn v4_mapped(addr: &IpAddrV4) -> Self {
        IpAddrV6 {
            scope_id: 0,
            addr: [0,0,0,0,0,0,0,0,0,0,0xFF,0xFF,
                   addr.addr[0],addr.addr[1],addr.addr[2],addr.addr[3]]
        }
    }

    /// Constructs a IP-v4 compatible address if the `addr` isn't in `0.0.0.0`, `0.0.0.1`.
    ///
    /// Ex. 192.168.0.1 => ::192.168.0.1
    pub fn v4_compatible(addr: &IpAddrV4) -> Option<Self> {
        if addr.addr[0] == 0 && addr.addr[1] == 0 && addr.addr[2] == 0
            && (addr.addr[3] == 0 || addr.addr[3] == 1)
        {
            None
        } else {
            Some(IpAddrV6 {
                scope_id: 0,
                addr: [0,0,0,0,0,0,0,0,0,0,0,0,
                       addr.addr[0],addr.addr[1],addr.addr[2],addr.addr[3]]
            })
        }
    }
}

impl fmt::Display for IpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ar: &[u16; 8] = unsafe { mem::transmute(&self.addr) };
        write!(f, "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
               u16::from_be(ar[0]), u16::from_be(ar[1]), u16::from_be(ar[2]), u16::from_be(ar[3]),
               u16::from_be(ar[4]), u16::from_be(ar[5]), u16::from_be(ar[6]), u16::from_be(ar[7]),)
    }
}

impl fmt::Debug for IpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
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
    assert!(LlAddr::default().addr == [0,0,0,0,0,0]);
    assert!(LlAddr::new(1,2,3,4,5,6).addr == [1,2,3,4,5,6]);
    assert!(LlAddr::new(1,2,3,4,5,6) == LlAddr::from_bytes(&[1,2,3,4,5,6]));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,3,4,5,7));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,3,4,6,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,3,5,0,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,4,0,0,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,3,0,0,0,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(2,0,0,0,0,0));
}

#[test]
fn test_ipaddr_v4() {
    assert!(IpAddrV4::default().addr == [0,0,0,0]);
    assert!(IpAddrV4::new(1,2,3,4).addr == [1,2,3,4]);
    assert!(IpAddrV4::new(1,2,3,4) == IpAddrV4::from_bytes(&[1,2,3,4]));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,2,3,5));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,2,4,0));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,3,0,0));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(2,0,0,0));
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
    assert_eq!(a, IpAddrV4::new(192,167,255, 156));
    a -= 256*10;
    assert_eq!(a, IpAddrV4::new(192,167,245,156));
}

#[test]
fn test_ipaddr_v6() {
    assert!(IpAddrV6::default().addr == [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);
    assert!(IpAddrV6::new(0x0102,0x0304,0x0506,0x0708,0x090a,0x0b0c,0x0d0e,0x0f10).addr
            == [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
    assert!(IpAddrV6::new(0x0102,0x0304,0x0506,0x0708,0x090a,0x0b0c,0x0d0e,0x0f10)
            == IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0));
    assert!(IpAddrV6::with_scope_id(0,0,0,0,0,0,0,0,100).get_scope_id() == 100);
    assert!(IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0) <
            IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,17], 0));
    assert!(IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0) <
            IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,16,00], 0));
    assert!(IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0) <
            IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,15,00,00], 0));
}
