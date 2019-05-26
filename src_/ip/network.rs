use super::{IpAddrV4, IpAddrV6, fmt_v6};

use std::fmt;
use std::mem;
use std::cmp::Ordering;

fn make_netmask_v6(len: u8) -> [u64; 2] {
    match len.cmp(&64) {
        Ordering::Less => [(!((1u64 << (64 - len)) - 1)).to_be(), 0],
        Ordering::Equal => [u64::max_value(), 0],
        Ordering::Greater => [u64::max_value(), (!((1u64 << (128 - len)) - 1)).to_be()],
    }
}

fn prefix_len(addr: &[u8]) -> u8 {
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

/// Implements Network IP version 4 style addresses.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct IpNetworkV4 {
    addr: IpAddrV4,
    len: u8,
}

impl IpNetworkV4 {
    /// Returns new IpNetworkV4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, IpNetworkV4};
    ///
    /// let ip = IpAddrV4::new(192, 168, 100, 1);
    /// assert_eq!(IpNetworkV4::new(ip, IpAddrV4::new(255,255,255,0)), IpNetworkV4::from(ip, 24));
    ///
    /// assert_eq!(IpNetworkV4::new(ip, IpAddrV4::new(0,0,0,255)), None);
    /// ```
    pub fn new(addr: IpAddrV4, netmask: IpAddrV4) -> Option<IpNetworkV4> {
        match prefix_len(&netmask.bytes) {
            0 => None,
            len => Some(IpNetworkV4 {
                addr: addr,
                len: len,
            }),
        }
    }

    /// Returns new IpNetworkV4.
    pub fn from(addr: IpAddrV4, prefix_len: u16) -> Option<IpNetworkV4> {
        if 1 <= prefix_len && prefix_len <= 32 {
            Some(IpNetworkV4 {
                addr: addr,
                len: prefix_len as u8,
            })
        } else {
            None
        }
    }

    /// Returns new IpNetworkV4.
    ///
    /// # Panics
    ///
    /// Panics if len == 0 or len > 32
    ///
    /// ```rust,no_run
    /// use asyncio::ip::{IpAddrV4, IpNetworkV4};
    ///
    /// unsafe { IpNetworkV4::from_unchecked(IpAddrV4::any(), 0); } // panic!
    /// ```
    pub unsafe fn from_unchecked(addr: IpAddrV4, prefix_len: u16) -> IpNetworkV4 {
        assert!(1 <= prefix_len && prefix_len <= 32);
        IpNetworkV4 {
            addr: addr,
            len: prefix_len as u8,
        }
    }

    /// Returns a address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, IpNetworkV4};
    ///
    /// let ip = IpNetworkV4::from(IpAddrV4::new(192, 168, 1, 1), 24).unwrap();
    /// assert_eq!(ip.address(), IpAddrV4::new(192, 168, 1, 1));
    /// ```
    pub fn address(&self) -> IpAddrV4 {
        self.addr.clone()
    }

    /// Returns a broadcast address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, IpNetworkV4};
    ///
    /// let ip = IpNetworkV4::from(IpAddrV4::new(192, 168, 1, 1), 24).unwrap();
    /// assert_eq!(ip.broadcast(), IpAddrV4::new(192, 168, 1, 255));
    /// ```

    pub fn broadcast(&self) -> IpAddrV4 {
        unsafe {
            let lhs: u32 = mem::transmute(self.network().bytes);
            let rhs: u32 = mem::transmute(self.netmask().bytes);
            IpAddrV4 { bytes: mem::transmute(lhs | !rhs) }
        }
    }

    /// Returns a canonical address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, IpNetworkV4};
    ///
    /// let ip = IpNetworkV4::from(IpAddrV4::new(192, 168, 1, 1), 24).unwrap();
    /// assert_eq!(ip.canonical(), IpNetworkV4::from(IpAddrV4::new(192, 168, 1, 0), 24).unwrap());
    /// ```
    pub fn canonical(&self) -> Self {
        IpNetworkV4 {
            addr: unsafe {
                let mask: u32 = mem::transmute(self.netmask().bytes);
                let addr: &u32 = mem::transmute(&self.addr.bytes);
                mem::transmute(*addr & mask)
            },
            len: self.len,
        }
    }

    pub fn hosts(&self) -> (IpAddrV4, IpAddrV4) {
        let mut addr = self.address();
        addr += 1;
        if self.is_host() {
            (self.address(), addr)
        } else {
            (addr, self.broadcast())
        }
    }

    pub fn is_host(&self) -> bool {
        self.len == 32
    }

    pub fn is_subnet_of(&self, other: &Self) -> bool {
        if other.len >= self.len {
            false
        } else {
            unsafe {
                let mask: u32 = mem::transmute(other.netmask().bytes);
                let lhs: u32 = mem::transmute_copy(&self.addr.bytes);
                let rhs: u32 = mem::transmute_copy(&other.addr.bytes);
                (lhs & mask) == (rhs & mask)
            }
        }
    }

    /// Returns a subnet mask.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, IpNetworkV4};
    ///
    /// let lo = IpNetworkV4::from(IpAddrV4::loopback(), 8).unwrap();
    /// assert_eq!(lo.netmask(), IpAddrV4::new(255,0,0,0));
    /// ```
    pub fn netmask(&self) -> IpAddrV4 {
        (0xffffffff << (32 - self.len)).into()
    }

    /// Returns a network address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, IpNetworkV4};
    ///
    /// let lo = IpNetworkV4::from(IpAddrV4::loopback(), 8).unwrap();
    /// assert_eq!(lo.network(), IpAddrV4::new(127,0,0,0));
    /// ```
    pub fn network(&self) -> IpAddrV4 {
        unsafe {
            let lhs: u32 = *(self.addr.bytes.as_ptr() as *const u32);
            let rhs: u32 = mem::transmute(self.netmask().bytes);
            IpAddrV4 { bytes: mem::transmute(lhs & rhs) }
        }
    }

    /// Returns a length of subnet mask.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV4, IpNetworkV4};
    ///
    /// let lo = IpNetworkV4::new(IpAddrV4::loopback(), IpAddrV4::new(254,0,0,0)).unwrap();
    /// assert_eq!(lo.prefix_len(), 7);
    /// ```
    pub fn prefix_len(&self) -> u16 {
        self.len as u16
    }
}

impl fmt::Display for IpNetworkV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.len)
    }
}

/// Implements Network IP version 6 style addresses.
pub struct IpNetworkV6 {
    bytes: [u8; 16],
    len: u8,
}

impl IpNetworkV6 {
    /// Returns new IpNetworkV6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV6, IpNetworkV6};
    ///
    /// assert!(IpNetworkV6::new(IpAddrV6::loopback(),
    ///                             IpAddrV6::new(0xffff,0xffff,0xffff,0xffff,0,0,0,0)).is_some());
    ///
    /// assert!(IpNetworkV6::new(IpAddrV6::loopback(),
    ///                             IpAddrV6::any()).is_none());
    /// ```
    pub fn new(addr: IpAddrV6, netmask: IpAddrV6) -> Option<Self> {
        match prefix_len(&netmask.bytes) {
            0 => None,
            len => Some(IpNetworkV6 {
                bytes: addr.bytes,
                len: len,
            }),
        }
    }

    pub fn from(addr: IpAddrV6, prefix_len: u16) -> Option<Self> {
        if 1 <= prefix_len && prefix_len <= 128 {
            Some(IpNetworkV6 {
                bytes: addr.bytes,
                len: prefix_len as u8,
            })
        } else {
            None
        }
    }

    /// Returns new IpNetworkV6.
    ///
    /// # Panics
    ///
    /// Panics if len == 0 or len > 128
    ///
    /// ```rust,no_run
    /// use asyncio::ip::{IpAddrV6, IpNetworkV6};
    ///
    /// IpNetworkV6::from(IpAddrV6::loopback(), 0);  // panic!
    /// ```
    pub unsafe fn from_unchecked(addr: IpAddrV6, len: u8) -> Self {
        assert!(1 <= len && len <= 128);
        IpNetworkV6 {
            bytes: addr.bytes,
            len: len,
        }
    }

    /// Returns a address.
    pub fn address(&self) -> IpAddrV6 {
        self.bytes.into()
    }

    pub fn canonical(&self) -> Self {
        IpNetworkV6 {
            bytes: unsafe {
                let mut lhs: [u64; 2] = make_netmask_v6(self.len);
                let rhs: &[u64; 2] = mem::transmute(&self.bytes);
                lhs[0] &= rhs[0];
                lhs[1] &= rhs[1];
                mem::transmute(lhs)
            },
            len: self.len,
        }
    }

    pub fn hosts(&self) -> (IpAddrV6, IpAddrV6) {
        let beg = self.network();
        let mut end = beg;
        end += 1 << (128 - self.len);
        (beg, end)
    }

    pub fn is_host(&self) -> bool {
        self.len == 128
    }

    pub fn is_subnet_of(&self, other: &Self) -> bool {
        if other.len >= self.len {
            false
        } else {
            unsafe {
                let mask: [u64; 2] = make_netmask_v6(other.len);
                let lhs: &[u64; 2] = mem::transmute(&self.bytes);
                let rhs: &[u64; 2] = mem::transmute(&other.bytes);
                ((lhs[0] & mask[0]) == (rhs[0] & mask[0])) &&
                    ((lhs[1] & mask[1]) == (rhs[1] & mask[1]))
            }
        }
    }

    /// Returns a subnet mask.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV6, IpNetworkV6};
    ///
    /// let lo = IpNetworkV6::from(IpAddrV6::loopback(), 64).unwrap();
    /// assert_eq!(lo.netmask(), IpAddrV6::new(0xffff,0xffff,0xffff,0xffff,0,0,0,0));
    /// ```
    pub fn netmask(&self) -> IpAddrV6 {
        IpAddrV6 {
            scope_id: 0,
            bytes: unsafe { mem::transmute(make_netmask_v6(self.len)) },
        }
    }

    /// Returns a network address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV6, IpNetworkV6};
    ///
    /// let lo = IpNetworkV6::from(IpAddrV6::loopback(), 64).unwrap();
    /// assert_eq!(lo.network(), IpAddrV6::any());
    /// ```
    pub fn network(&self) -> IpAddrV6 {
        unsafe {
            let mut lhs: [u64; 2] = make_netmask_v6(self.len);
            let rhs: &[u64; 2] = mem::transmute(&self.bytes);
            lhs[0] &= rhs[0];
            lhs[1] &= rhs[1];
            IpAddrV6 {
                scope_id: 0,
                bytes: mem::transmute(lhs),
            }
        }
    }


    /// Returns a length of subnet mask.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpAddrV6, IpNetworkV6};
    ///
    /// let lo = IpNetworkV6::new(IpAddrV6::loopback(),
    ///   IpAddrV6::new(0xffff,0xffff,0xffff,0xfffe,0,0,0,0)).unwrap();
    /// assert_eq!(lo.prefix_len(), 63)
    /// ```
    pub fn prefix_len(&self) -> u16 {
        self.len as u16
    }
}

impl fmt::Display for IpNetworkV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(fmt_v6(&self.bytes, f));
        write!(f, "/{}", self.len)
    }
}

#[test]
fn test_prefix_len() {
    assert_eq!(prefix_len(&[255, 255, 255, 0]), 24);
    assert_eq!(prefix_len(&[255, 255, 255, 255]), 32);
    assert_eq!(prefix_len(&[255, 255, 254, 0]), 23);
    assert_eq!(prefix_len(&[255, 255, 255, 254]), 31);
    assert_eq!(prefix_len(&[128, 0, 0, 0]), 1);

    assert_eq!(prefix_len(&[0, 0, 0, 0]), 0);
    assert_eq!(prefix_len(&[1, 1, 1, 1]), 0);
    assert_eq!(prefix_len(&[128, 1, 1, 1]), 0);
}

#[test]
fn test_ip_network_v4() {
    let ip = IpNetworkV4::new(
        IpAddrV4::new(192, 168, 0, 1),
        IpAddrV4::new(255, 255, 255, 0),
    ).unwrap();
    assert_eq!(ip.network(), IpAddrV4::new(192, 168, 0, 0));
    assert_eq!(ip.netmask(), IpAddrV4::new(255, 255, 255, 0));
    assert_eq!(ip.prefix_len(), 24);

    let ip = IpNetworkV4::new(
        IpAddrV4::new(192, 168, 255, 1),
        IpAddrV4::new(255, 255, 240, 0),
    ).unwrap();
    assert_eq!(ip.network(), IpAddrV4::new(192, 168, 240, 0));
    assert_eq!(ip.netmask(), IpAddrV4::new(255, 255, 240, 0));
    assert_eq!(ip.prefix_len(), 20);
}

#[test]
fn test_ip_network_canonical_v4() {
    let ip1 = IpNetworkV4::from(IpAddrV4::new(172, 16, 1, 1), 16).unwrap();
    let ip2 = IpNetworkV4::from(IpAddrV4::new(172, 16, 2, 1), 16).unwrap();
    assert_eq!(ip1.canonical(), ip2.canonical());
}

#[test]
fn test_ip_network_v4_hosts() {
    let (ip1, ip2) = IpNetworkV4::from(IpAddrV4::new(192, 168, 0, 0), 24)
        .unwrap()
        .hosts();
    assert_eq!(ip1, IpAddrV4::new(192, 168, 0, 1));
    assert_eq!(ip2, IpAddrV4::new(192, 168, 0, 255));
}

#[test]
#[should_panic]
fn test_panic_ip_network_v4() {
    unsafe { IpNetworkV4::from_unchecked(IpAddrV4::loopback(), 0) };
}

#[test]
fn test_ip_network_v4_format() {
    let ip = IpNetworkV4::new(
        IpAddrV4::new(192, 168, 0, 1),
        IpAddrV4::new(255, 255, 255, 0),
    ).unwrap();
    assert_eq!(format!("{}", ip), "192.168.0.1/24");
    assert_eq!(format!("{}", ip.canonical()), "192.168.0.0/24");
}

#[test]
fn test_ip_network_v6_half() {
    let netmask = IpAddrV6::new(0xffff, 0xffff, 0xffff, 0xffff, 0, 0, 0, 0);
    let ip = IpNetworkV6::new(
        IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0xdead, 0xbeaf),
        netmask,
    ).unwrap();
    assert_eq!(ip.network(), IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0, 0));
    assert_eq!(ip.netmask(), netmask);
    assert_eq!(ip.prefix_len(), 64);
}

#[test]
fn test_ip_network_v6_long() {
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
    let ip = IpNetworkV6::new(
        IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0xdead, 0xbeaf),
        netmask,
    ).unwrap();
    assert_eq!(
        ip.network(),
        IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0xdead, 0xbea0)
    );
    assert_eq!(ip.netmask(), netmask);
    assert_eq!(ip.prefix_len(), 124);
}

#[test]
fn test_ip_network_v6_short() {
    let netmask = IpAddrV6::new(0xfe00, 0, 0, 0, 0, 0, 0, 0);
    let ip = IpNetworkV6::new(
        IpAddrV6::new(0x2001, 0, 0, 0, 0, 0, 0xdead, 0xbeaf),
        netmask,
    ).unwrap();
    assert_eq!(ip.network(), IpAddrV6::new(0x2000, 0, 0, 0, 0, 0, 0, 0));
    assert_eq!(ip.netmask(), netmask);
    assert_eq!(ip.prefix_len(), 7);
}

#[test]
#[should_panic]
fn test_panic_ip_network_v6() {
    unsafe { IpNetworkV6::from_unchecked(IpAddrV6::loopback(), 0) };
}

#[test]
fn test_ip_network_v6_format() {
    let ip = IpNetworkV6::from(IpAddrV6::loopback(), 64).unwrap();
    assert_eq!(format!("{}", ip), "::1/64");

    let ip = IpNetworkV6::from(IpAddrV6::new(0xdead, 0xbeaf, 0, 0, 0, 0, 0, 0), 32).unwrap();
    assert_eq!(format!("{}", ip), "dead:beaf::/32");
}
