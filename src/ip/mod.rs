use std::io;
use std::fmt;
use std::mem;
use std::cmp;
use std::hash;
use std::marker::PhantomData;
use libc::{AF_INET, AF_INET6, sockaddr, sockaddr_in, sockaddr_in6, sockaddr_storage};
use traits::{Protocol, SockAddr};
use io_service::{Handler, FromRawFd};
use sa_ops::{SockAddrImpl, sockaddr_eq, sockaddr_cmp, sockaddr_hash};

/// The endpoint of internet protocol.
#[derive(Clone)]
pub struct IpEndpoint<P: Protocol> {
    ss: SockAddrImpl<sockaddr_storage>,
    _marker: PhantomData<P>,
}

impl<P: Protocol> IpEndpoint<P> {
    /// Returns a IpEndpoint from IP address and port number.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::{IpEndpoint, IpAddrV4, Tcp};
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV4::loopback(), 80);
    /// ```
    pub fn new<T>(addr: T, port: u16) -> Self
        where T: ToEndpoint<P>,
    {
        addr.to_endpoint(port)
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
        self.ss.ss_family as i32 == AF_INET
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
        self.ss.ss_family as i32 == AF_INET6
    }

    /// Returns a IP address.
    pub fn addr(&self) -> IpAddr {
        match self.ss.ss_family as i32 {
            AF_INET => {
                let sin: &sockaddr_in = unsafe { self.ss.as_sockaddr() };
                let bytes: [u8; 4] = unsafe { mem::transmute(sin.sin_addr) };
                IpAddr::V4(IpAddrV4::from(bytes))
            },
            AF_INET6  => {
                let sin6: &sockaddr_in6 = unsafe { self.ss.as_sockaddr() };
                let bytes: [u8; 16] = unsafe { mem::transmute(sin6.sin6_addr) };
                IpAddr::V6(IpAddrV6::from(bytes, sin6.sin6_scope_id))
            },
            _ => panic!("Invalid address family ({}).", self.ss.ss_family),
        }
    }

    /// Returns a port number.
    pub fn port(&self) -> u16 {
        let sin: &sockaddr_in = unsafe { self.ss.as_sockaddr() };
        u16::from_be(sin.sin_port)
    }

    fn from_v4(addr: &IpAddrV4, port: u16) -> IpEndpoint<P> {
        let mut ep = IpEndpoint {
            ss: SockAddrImpl::new(AF_INET, mem::size_of::<sockaddr_in>()),
            _marker: PhantomData,
        };
        {
            let sin: &mut sockaddr_in = unsafe { ep.ss.as_mut_sockaddr() };
            sin.sin_port = port.to_be();
            sin.sin_addr = unsafe { mem::transmute(addr.as_bytes().clone()) };
            sin.sin_zero = [0; 8];
        }
        ep
    }

    fn from_v6(addr: &IpAddrV6, port: u16) -> IpEndpoint<P> {
        let mut ep = IpEndpoint {
            ss: SockAddrImpl::new(AF_INET6, mem::size_of::<sockaddr_in6>()),
            _marker: PhantomData,
        };
        {
            let sin6: &mut sockaddr_in6 = unsafe { ep.ss.as_mut_sockaddr() };
            sin6.sin6_port = port.to_be();
            sin6.sin6_flowinfo = 0;
            sin6.sin6_addr = unsafe { mem::transmute(addr.as_bytes().clone()) };
            sin6.sin6_scope_id = addr.get_scope_id();
        }
        ep
    }
}

impl<P: Protocol> SockAddr for IpEndpoint<P> {
    fn as_sockaddr(&self) -> &sockaddr {
        unsafe { self.ss.as_sockaddr() }
    }

    fn as_mut_sockaddr(&mut self) -> &mut sockaddr {
        unsafe { self.ss.as_mut_sockaddr() }
    }

    fn capacity(&self) -> usize {
        self.ss.capacity()
    }

    fn size(&self) -> usize {
        self.ss.size()
    }

    unsafe fn resize(&mut self, size: usize) {
        debug_assert!(size <= self.capacity());
        self.ss.resize(size)
    }
}

impl<P: Protocol> Eq for IpEndpoint<P> {
}

impl<P: Protocol> PartialEq for IpEndpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        sockaddr_eq(self, other)
    }
}

impl<P: Protocol> Ord for IpEndpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        sockaddr_cmp(self, other)
    }
}

impl<P: Protocol> PartialOrd for IpEndpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Protocol> hash::Hash for IpEndpoint<P> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        sockaddr_hash(self, state)
    }
}

impl<P: Protocol> fmt::Display for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "{}:{}", addr, self.port()),
            IpAddr::V6(addr) => write!(f, "[{}]:{}", addr, self.port()),
        }
    }
}

impl<P: Protocol> fmt::Debug for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// A category of an internet protocol.
pub trait IpProtocol : Protocol {

    /// Returns true if the protocol associated IP-v4 address.
    fn is_v4(&self) -> bool;

    /// Returns true if the protocol associated IP-v6 address.
    fn is_v6(&self) -> bool;

    /// Returns a IP-v4 protocol.
    fn v4() -> Self;

    /// Returns a IP-v6 protocol.
    fn v6() -> Self;

    #[doc(hidden)]
    type Socket : FromRawFd<Self>;

    #[doc(hidden)]
    fn connect(soc: &Self::Socket, ep: &IpEndpoint<Self>) -> io::Result<()>;

    #[doc(hidden)]
    fn async_connect<F: Handler<()>>(soc: &Self::Socket, ep: &IpEndpoint<Self>, handler: F) -> F::Output;
}

/// Provides conversion to a IP-endpoint.
pub trait ToEndpoint<P: Protocol> {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P>;
}

impl<P: IpProtocol> ToEndpoint<P> for P {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P> {
        if self.is_v4() {
            IpEndpoint::new(IpAddrV4::any(), port)
        } else if self.is_v6() {
            IpEndpoint::new(IpAddrV6::any(), port)
        } else {
            unreachable!();
        }
    }
}

impl<P: Protocol> ToEndpoint<P> for IpAddrV4 {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from_v4(&self, port)
    }
}

impl<P: Protocol> ToEndpoint<P> for IpAddrV6 {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from_v6(&self, port)
    }
}

impl<P: Protocol> ToEndpoint<P> for IpAddr {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P> {
        match self {
            IpAddr::V4(addr) => IpEndpoint::from_v4(&addr, port),
            IpAddr::V6(addr) => IpEndpoint::from_v6(&addr, port),
        }
    }
}

impl<'a, P: IpProtocol> ToEndpoint<P> for &'a P {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P> {
        if self.is_v4() {
            IpEndpoint::new(IpAddrV4::any(), port)
        } else if self.is_v6() {
            IpEndpoint::new(IpAddrV6::any(), port)
        } else {
            unreachable!();
        }
    }
}

impl<'a, P: Protocol> ToEndpoint<P> for &'a IpAddrV4 {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from_v4(self, port)
    }
}

impl<'a, P: Protocol> ToEndpoint<P> for &'a IpAddrV6 {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P> {
        IpEndpoint::from_v6(self, port)
    }
}

impl<'a, P: Protocol> ToEndpoint<P> for &'a IpAddr {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P> {
        match self {
            &IpAddr::V4(ref addr) => IpEndpoint::from_v4(addr, port),
            &IpAddr::V6(ref addr) => IpEndpoint::from_v6(addr, port),
        }
    }
}

mod addr;
pub use self::addr::*;

mod hostname;
pub use self::hostname::*;

mod resolver;
pub use self::resolver::*;

mod tcp;
pub use self::tcp::*;

mod udp;
pub use self::udp::*;

mod icmp;
pub use self::icmp::*;

mod option;
pub use self::option::*;

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
