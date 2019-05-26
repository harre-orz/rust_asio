//

use super::{IpAddr, IpAddrV4, IpAddrV6};
use libc;
use socket_base::Endpoint;
use std::marker::PhantomData;
use std::mem;
use std::fmt;
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};

union SockAddr {
    v4: libc::sockaddr_in,
    v6: libc::sockaddr_in6,
}

pub struct IpEndpoint<P> {
    inner: SockAddr,
    _marker: PhantomData<P>,
}

impl<P> IpEndpoint<P> {
    fn family(&self) -> i32 {
        unsafe { self.inner.v4 }.sin_family as _
    }

    /// Returns a IpEndpoint from IP address and port number.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{IpEndpoint, IpAddrV4, Tcp};
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV4::loopback(), 80);
    /// ```
    pub fn new<T>(addr: T, port: u16) -> Self
    where
        T: Into<IpAddr>,
    {
        match addr.into() {
            IpAddr::V4(x) => IpEndpoint::v4(x, port),
            IpAddr::V6(x) => IpEndpoint::v6(x, port),
        }
    }

    pub const fn v4(addr: IpAddrV4, port: u16) -> Self {
        IpEndpoint {
            inner: SockAddr {
                v4: libc::sockaddr_in {
                    sin_family: libc::AF_INET as _,
                    sin_port: port.to_be(),
                    sin_addr: addr.into_in_addr(),
                    sin_zero: [0; 8],
                },
            },
            _marker: PhantomData,
        }
    }

    pub const fn v6(addr: IpAddrV6, port: u16) -> Self {
        let scope_id = addr.scope_id();
        IpEndpoint {
            inner: SockAddr {
                v6: libc::sockaddr_in6 {
                    sin6_family: libc::AF_INET6 as _,
                    sin6_port: port.to_be(),
                    sin6_addr: addr.into_in6_addr(),
                    sin6_flowinfo: 0,
                    sin6_scope_id: scope_id,
                },
            },
            _marker: PhantomData,
        }
    }

    /// Returns true if this is IpEndpoint of IP-v4 address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{IpEndpoint, IpAddrV4, IpAddrV6, Tcp};
    ///
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV4::loopback(), 80);
    /// assert_eq!(ep.is_v4(), true);
    ///
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV6::loopback(), 80);
    /// assert_eq!(ep.is_v4(), false);
    /// ```
    pub fn is_v4(&self) -> bool {
        self.family() == libc::AF_INET
    }

    /// Returns true if this is IpEndpoint of IP-v6 address.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{IpEndpoint, IpAddrV4, IpAddrV6, Tcp};
    ///
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV4::loopback(), 80);
    /// assert_eq!(ep.is_v6(), false);
    ///
    /// let ep: IpEndpoint<Tcp> = IpEndpoint::new(IpAddrV6::loopback(), 80);
    /// assert_eq!(ep.is_v6(), true);
    /// ```
    pub fn is_v6(&self) -> bool {
        self.family() == libc::AF_INET6
    }

    /// Returns a IP address.
    pub fn addr(&self) -> IpAddr {
        match self.family() {
            libc::AF_INET => IpAddr::V4(unsafe { self.inner.v4 }.sin_addr.into()),
            libc::AF_INET6 => IpAddr::V6(unsafe { self.inner.v6 }.sin6_addr.into()),
            _ => unreachable!(),
        }
    }

    /// Returns a port number.
    pub fn port(&self) -> u16 {
        u16::from_be(unsafe { self.inner.v4 }.sin_port)
    }
}

impl<P> Endpoint<P> for IpEndpoint<P> {
    fn as_ptr(&self) -> *const libc::sockaddr {
        unsafe { &self.inner.v4 as *const _ as *const _ }
    }

    fn as_mut_ptr(&mut self) -> *mut libc::sockaddr {
        unsafe { &mut self.inner.v4 as *mut _ as *mut _ }
    }

    fn capacity(&self) -> libc::socklen_t {
        mem::size_of::<SockAddr>() as _
    }

    fn size(&self) -> libc::socklen_t {
        match self.family() {
            libc::AF_INET => mem::size_of::<libc::sockaddr_in>() as _,
            libc::AF_INET6 => mem::size_of::<libc::sockaddr_in6>() as _,
            _ => unreachable!(),
        }
    }

    unsafe fn resize(&mut self, len: libc::socklen_t) {
        debug_assert_eq!(len, self.size())
    }
}

impl<P> From<SocketAddr> for IpEndpoint<P> {
    fn from(sa: SocketAddr) -> Self {
        match sa {
            SocketAddr::V4(sa4) => sa4.into(),
            SocketAddr::V6(sa6) => sa6.into(),
        }
    }
}

impl<P> From<SocketAddrV4> for IpEndpoint<P> {
    fn from(sa4: SocketAddrV4) -> Self {
        IpEndpoint::v4(sa4.ip().into(), sa4.port())
    }
}

impl<P> From<SocketAddrV6> for IpEndpoint<P> {
    fn from(sa6: SocketAddrV6) -> Self {
        IpEndpoint::v6(sa6.ip().into(), sa6.port())
    }
}

impl<P> From<libc::sockaddr_in> for IpEndpoint<P> {
    fn from(sin: libc::sockaddr_in) -> Self {
        IpEndpoint {
            inner: SockAddr { v4: sin },
            _marker: PhantomData,
        }
    }
}

impl<P> From<libc::sockaddr_in6> for IpEndpoint<P> {
    fn from(sin6: libc::sockaddr_in6) -> Self {
        IpEndpoint {
            inner: SockAddr { v6: sin6 },
            _marker: PhantomData,
        }
    }
}

impl<P> fmt::Debug for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "IpEndpoint {{ addr = {}, port = {} }}", addr, self.port()),
            IpAddr::V6(addr) => write!(f, "IpEndpoint {{ addr = {}, port = {} }}", addr, self.port()),
        }
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
    assert_eq!(ep.size() as usize, mem::size_of::<libc::sockaddr_in>());
}

#[test]
fn test_endpoint_v6() {
    use ip::TcpEndpoint;

    let ep = TcpEndpoint::new(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8), 10);
    assert!(ep.is_v6());
    assert!(!ep.is_v4());
    assert_eq!(ep.addr(), IpAddr::V6(IpAddrV6::new(1, 2, 3, 4, 5, 6, 7, 8)));
    assert_eq!(ep.port(), 10);
    assert_eq!(ep.size() as usize, mem::size_of::<libc::sockaddr_in6>());
}
