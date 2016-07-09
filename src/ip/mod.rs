use std::io;
use std::fmt;
use std::mem;
use std::ptr;
use std::cmp;
use std::sync::Arc;
use std::marker::PhantomData;
use {IoService, UnsafeThreadableCell, Protocol, AsSockAddr};
use ops::*;

mod addr;
pub use self::addr::*;

/// The endpoint of internet protocol.
#[derive(Clone)]
pub struct IpEndpoint<P: Protocol> {
    len: usize,
    ss: sockaddr_storage,
    maker: PhantomData<P>,
}

impl<P: Protocol> IpEndpoint<P> {
    pub fn new<T: ToEndpoint<P>>(addr: T, port: u16) -> Self {
        addr.to_endpoint(port)
    }

    pub fn is_v4(&self) -> bool {
        self.ss.ss_family == AF_INET as u16
    }

    pub fn is_v6(&self) -> bool {
        self.ss.ss_family == AF_INET6 as u16
    }

    pub fn addr(&self) -> IpAddr {
        match self.ss.ss_family as i32 {
            AF_INET => {
                let sin: &sockaddr_in = unsafe { mem::transmute(&self.ss) };
                IpAddr::V4(IpAddrV4::from_bytes(unsafe { mem::transmute(&sin.sin_addr) }))
            },
            AF_INET6  => {
                let sin6: &sockaddr_in6 = unsafe { mem::transmute(&self.ss) };
                IpAddr::V6(IpAddrV6::from_bytes(unsafe { mem::transmute(&sin6.sin6_addr) }, sin6.sin6_scope_id))
            },
            _ => panic!("Invalid domain ({}).", self.ss.ss_family),
        }
    }

    pub fn port(&self) -> u16 {
        let sin: &sockaddr_in = unsafe { mem::transmute(&self.ss) };
        u16::from_be(sin.sin_port)
    }

    fn default() -> IpEndpoint<P> {
        IpEndpoint {
            len: mem::size_of::<sockaddr_storage>(),
            ss: unsafe { mem::zeroed() },
            maker: PhantomData,
        }
    }

    fn from_v4(addr: &IpAddrV4, port: u16) -> IpEndpoint<P> {
        let mut ep = IpEndpoint::default();
        let sin: &mut sockaddr_in = unsafe { mem::transmute(&mut ep.ss) };
        sin.sin_family = AF_INET as u16;
        sin.sin_port = port.to_be();
        unsafe {
            let src: *const u32 = mem::transmute(addr.as_bytes().as_ptr());
            let dst: *mut u32 = mem::transmute(&mut sin.sin_addr);
            ptr::copy(src, dst, 1);
        }
        ep
    }

    fn from_v6(addr: &IpAddrV6, port: u16) -> IpEndpoint<P> {
        let mut ep = IpEndpoint::default();
        let sin6: &mut sockaddr_in6 = unsafe { mem::transmute(&mut ep.ss) };
        sin6.sin6_family = AF_INET6 as u16;
        sin6.sin6_port = port.to_be();
        sin6.sin6_scope_id = addr.get_scope_id();
        unsafe {
            let src: *const u64 = mem::transmute(addr.as_bytes().as_ptr());
            let dst: *mut u64 = mem::transmute(&mut sin6.sin6_addr);
            ptr::copy(src, dst, 2);
        }
        ep
    }
}

impl<P: Protocol> AsSockAddr for IpEndpoint<P> {
    type SockAddr = sockaddr_storage;

    fn as_sockaddr(&self) -> &Self::SockAddr {
        &self.ss
    }

    fn as_mut_sockaddr(&mut self) -> &mut Self::SockAddr {
        &mut self.ss
    }

    fn size(&self) -> usize {
        self.len
    }

    fn resize(&mut self, size: usize) {
        self.len = cmp::min(size, self.capacity())
    }

    fn capacity(&self) -> usize {
        mem::size_of::<Self::SockAddr>()
    }
}

impl<P: Protocol> Eq for IpEndpoint<P> {
}

impl<P: Protocol> PartialEq for IpEndpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            c_memcmp(
                mem::transmute(&self.ss),
                mem::transmute(&other.ss),
                mem::size_of::<sockaddr_storage>()
            ) == 0
        }
    }
}

impl<P: Protocol> Ord for IpEndpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let cmp = unsafe {
            c_memcmp(
                mem::transmute(&self.ss),
                mem::transmute(&other.ss),
                mem::size_of::<sockaddr_storage>()
            )
        };
        if cmp == 0 {
            cmp::Ordering::Equal
        } else if cmp < 0 {
            cmp::Ordering::Less
        } else {
            cmp::Ordering::Greater
        }
    }
}

impl<P: Protocol> PartialOrd for IpEndpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
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

/// Provides conversion to a IP-endpoint.
pub trait ToEndpoint<P: Protocol> {
    fn to_endpoint(self, port: u16) -> IpEndpoint<P>;
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

/// An entry produced by a resolver.
pub struct Resolver<P: Protocol, S> {
    io: IoService,
    socket: UnsafeThreadableCell<Option<Arc<S>>>,
    marker: PhantomData<P>,
}

/// An iterator over the entries produced by a resolver.
pub struct ResolverIter<'a, P: Protocol> {
    base: *mut addrinfo,
    ai: *mut addrinfo,
    marker: PhantomData<&'a P>,
}

impl<'a, P: Protocol> ResolverIter<'a, P> {
    fn _new(pro: P, host: &str, port: &str, flags: i32) -> io::Result<ResolverIter<'a, P>> {
        let base = try!(unsafe { getaddrinfo(pro, host, port, flags) });
        Ok(ResolverIter {
            base: base,
            ai: base,
            marker: PhantomData,
        })
    }

    unsafe fn into_inner(mut self) -> UnsafeResolverIter<P> {
        let sender = UnsafeResolverIter {
            base: self.base,
            ai: self.ai,
            marker: PhantomData,
        };
        self.base = ptr::null_mut();
        sender
    }
}

struct UnsafeResolverIter<P: Protocol> {
    base: *mut addrinfo,
    ai: *mut addrinfo,
    marker: PhantomData<P>,
}

fn host_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Host not found")
}

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
    assert!(ep.addr() == IpAddr::V4(IpAddrV4::new(1,2,3,4)));
    assert!(ep.port() == 10);
    assert!(!ep.is_v6());
}

#[test]
fn test_endpoint_v6() {
    let ep = TcpEndpoint::new(IpAddrV6::new(1,2,3,4,5,6,7,8), 10);
    assert!(ep.is_v6());
    assert!(ep.addr() == IpAddr::V6(IpAddrV6::new(1,2,3,4,5,6,7,8)));
    assert!(ep.port() == 10);
    assert!(!ep.is_v4());
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
