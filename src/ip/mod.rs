use ffi::*;
use core::IoContext;
use prelude::Protocol;

use std::io;
use std::fmt;
use std::mem;
use std::marker::PhantomData;

pub trait IpProtocol : Protocol + Eq {
    fn v4() -> Self;

    fn v6() -> Self;

    fn from_ai(*mut addrinfo) -> Option<Self::Endpoint>;
}

/// The endpoint of internet protocol.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IpEndpoint<P> {
    ss: SockAddr<sockaddr_storage>,
    _marker: PhantomData<P>,
}

impl<P> IpEndpoint<P> {
    /// Returns a IpEndpoint from IP address and port number.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpEndpoint, IpAddrV4, Tcp};
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV4::loopback(), 80);
    /// ```
    pub fn new<T>(addr: T, port: u16) -> Self
        where T: IntoEndpoint<P>,
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
            _ => panic!("Invalid address family ({}).", self.ss.sa.ss_family),
        }
    }

    /// Returns a port number.
    pub fn port(&self) -> u16 {
        let sin = unsafe { &*(&self.ss.sa as *const _ as *const sockaddr_in) };
        u16::from_be(sin.sin_port)
    }

    fn from_v4(addr: &IpAddrV4, port: u16) -> IpEndpoint<P> {
        let mut ep = IpEndpoint {
            ss: SockAddr::new(AF_INET, mem::size_of::<sockaddr_in>() as u8),
            _marker: PhantomData,
        };
        unsafe {
            let sin = &mut *(&mut ep.ss.sa as *mut _ as *mut sockaddr_in);
            sin.sin_port = port.to_be();
            sin.sin_addr = mem::transmute(addr.as_bytes().clone());
            sin.sin_zero = [0; 8];
        }
        ep
    }

    fn from_v6(addr: &IpAddrV6, port: u16) -> IpEndpoint<P> {
        let mut ep = IpEndpoint {
            ss: SockAddr::new(AF_INET6, mem::size_of::<sockaddr_in6>() as u8),
            _marker: PhantomData,
        };
        unsafe {
            let sin6 = &mut *(&mut ep.ss.sa as *mut _ as *mut sockaddr_in6);
            sin6.sin6_port = port.to_be();
            sin6.sin6_flowinfo = 0;
            sin6.sin6_addr = mem::transmute(addr.as_bytes().clone());
            sin6.sin6_scope_id = addr.get_scope_id();
        }
        ep
    }
}

impl<P: IpProtocol> IpEndpoint<P> {
    pub fn protocol(&self) -> P {
        if self.is_v4() {
            return P::v4()
        }
        if self.is_v6() {
            return P::v6()
        }
        unreachable!("Invalid address family ({}).", self.ss.sa.ss_family);
    }
}

impl<P: IpProtocol + fmt::Debug> fmt::Debug for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "{:?}:{}:{}", self.protocol(), addr, self.port()),
            IpAddr::V6(addr) => write!(f, "{:?}:[{}]:{}", self.protocol(), addr, self.port()),
        }
    }
}

/// Provides conversion to a IP-endpoint.
pub trait IntoEndpoint<P> {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P>;
}

impl<P: IpProtocol> IntoEndpoint<P> for P {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        if &self == &P::v4() {
            return IpEndpoint::from_v4(&IpAddrV4::any(), port)
        }
        if &self == &P::v6() {
            return IpEndpoint::from_v6(&IpAddrV6::any(), port)
        }
        unreachable!("Invalid protocol");
    }
}

impl<P> IntoEndpoint<P> for IpAddrV4 {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from_v4(&self, port)
    }
}

impl<P> IntoEndpoint<P> for IpAddrV6 {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from_v6(&self, port)
    }
}

impl<P> IntoEndpoint<P> for IpAddr {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        match self {
            IpAddr::V4(addr) => IpEndpoint::from_v4(&addr, port),
            IpAddr::V6(addr) => IpEndpoint::from_v6(&addr, port),
        }
    }
}

impl<'a, P> IntoEndpoint<P> for &'a IpAddrV4 {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from_v4(self, port)
    }
}

impl<'a, P> IntoEndpoint<P> for &'a IpAddrV6 {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from_v6(self, port)
    }
}

impl<'a, P> IntoEndpoint<P> for &'a IpAddr {
    fn into_endpoint(self, port: u16) -> IpEndpoint<P> {
        match self {
            &IpAddr::V4(ref addr) => IpEndpoint::from_v4(addr, port),
            &IpAddr::V6(ref addr) => IpEndpoint::from_v6(addr, port),
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
    gethostname().map_err(error)
}

mod addr;
pub use self::addr::*;

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
fn test_host_name() {
    let ctx = &IoContext::new().unwrap();
    host_name(ctx).unwrap();
}

#[test]
fn test_endpoint_v4() {
    let ep = UdpEndpoint::new(IpAddrV4::new(1,2,3,4), 10);
    assert!(ep.is_v4());
    assert!(!ep.is_v6());
    assert_eq!(ep.addr(), IpAddr::V4(IpAddrV4::new(1,2,3,4)));
    assert_eq!(ep.port(), 10);
}

#[test]
fn test_endpoint_v6() {
    let ep = TcpEndpoint::new(IpAddrV6::new(1,2,3,4,5,6,7,8), 10);
    assert!(ep.is_v6());
    assert!(!ep.is_v4());
    assert_eq!(ep.addr(), IpAddr::V6(IpAddrV6::new(1,2,3,4,5,6,7,8)));
    assert_eq!(ep.port(), 10);
}

#[test]
fn test_endpoint_cmp() {
    let a = IcmpEndpoint::new(IpAddrV6::new(1,2,3,4,5,6,7,8), 10);
    let b = IcmpEndpoint::new(IpAddrV6::with_scope_id(1,2,3,4,5,6,7,8,1), 10);
    let c = IcmpEndpoint::new(IpAddrV6::new(1,2,3,4,5,6,7,8), 11);
    assert!(a == a && b == b && c == c);
    assert!(a != b && b != c);
    assert!(a < b);
    assert!(b < c);
}
