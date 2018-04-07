use super::{IpProtocol, IpAddrV4, IpAddrV6, IpAddr};
use ffi::{AF_INET, AF_INET6, SockAddr, socklen_t, sockaddr, sockaddr_in, sockaddr_in6, sockaddr_storage};
use prelude::Endpoint;

use std::fmt;
use std::mem;
use std::marker::PhantomData;

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

    #[doc(hidden)]
    pub fn from_ss(ss: SockAddr<sockaddr_storage>) -> Self {
        IpEndpoint {
            ss: ss,
            _marker: PhantomData,
        }
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

impl<P: IpProtocol> fmt::Display for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "{}:{}", addr, self.port()),
            IpAddr::V6(addr) => write!(f, "[{}]:{}", addr, self.port()),
        }
    }
}

impl<P: IpProtocol> fmt::Debug for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "{}:{}", addr, self.port()),
            IpAddr::V6(addr) => write!(f, "[{}]:{}", addr, self.port()),
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
            sin6.sin6_scope_id = t.0.scope_id();
            sin6.sin6_addr = mem::transmute(t.0.bytes);
        }
        ep
    }
}

#[test]
fn test_endpoint_v4() {
    use ip::UdpEndpoint;

    let ep = UdpEndpoint::new(IpAddrV4::new(1, 2, 3, 4), 10);
    assert!(ep.is_v4());
    assert!(!ep.is_v6());
    assert_eq!(ep.addr(), IpAddr::V4(IpAddrV4::new(1, 2, 3, 4)));
    assert_eq!(ep.port(), 10);
}

#[test]
fn test_endpoint_v6() {
    use ip::TcpEndpoint;

    let ep = TcpEndpoint::new(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8), 10);
    assert!(ep.is_v6());
    assert!(!ep.is_v4());
    assert_eq!(ep.addr(), IpAddr::V6(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8)));
    assert_eq!(ep.port(), 10);
}

#[test]
fn test_endpoint_cmp() {
    use ip::IcmpEndpoint;

    let a = IcmpEndpoint::new(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8), 10);
    let b = IcmpEndpoint::new(IpAddrV6::with_scope_id(1, 2, 3, 4, 5, 6, 7, 8, 1), 10);
    let c = IcmpEndpoint::new(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8), 11);
    assert!(a == a && b == b && c == c);
    assert!(a != b && b != c);
    assert!(a < b);
    assert!(b < c);
}
