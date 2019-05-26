//

use super::{
    IpAddr, IpAddrV4, IpAddrV6, IpEndpoint, MulticastEnableLoopback, MulticastHops, MulticastJoinGroup,
    MulticastLeaveGroup, NoDelay, OutboundInterface, Resolver, ResolverIter, ResolverQuery, UnicastHops, V6Only,
};

use executor::IoContext;
use libc;
use socket_base::{get_sockopt, set_sockopt, GetSocketOption, Protocol, SetSocketOption};
use socket_listener::SocketListener;
use std::io;
use std::mem;
use stream_socket::StreamSocket;

/// Transmission Control Protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tcp {
    family: i32,
}

impl Tcp {
    pub const fn v4() -> Self {
        Tcp { family: libc::AF_INET }
    }

    pub const fn v6() -> Self {
        Tcp { family: libc::AF_INET6 }
    }
}

impl Protocol for Tcp {
    type Endpoint = IpEndpoint<Self>;
    type Socket = StreamSocket<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_STREAM as i32
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl From<Tcp> for IpAddr {
    fn from(tcp: Tcp) -> Self {
        match tcp.family {
            libc::AF_INET => IpAddr::V4(IpAddrV4::default()),
            libc::AF_INET6 => IpAddr::V6(IpAddrV6::default()),
            _ => unreachable!(),
        }
    }
}

impl Resolver<Tcp> {
    pub fn tcp(ctx: &IoContext) -> Self {
        Resolver::new(
            ctx,
            Tcp {
                family: libc::AF_UNSPEC,
            },
        )
    }

    pub fn resolve<Q>(&self, host: Q, port: u16) -> io::Result<ResolverIter<Tcp>>
    where
        Q: Into<ResolverQuery>,
    {
        self.addrinfo(host, port, 0)
    }
}

pub type TcpEndpoint = IpEndpoint<Tcp>;
pub type TcpListener = SocketListener<Tcp>;
pub type TcpResolver = Resolver<Tcp>;
pub type TcpSocket = StreamSocket<Tcp>;

impl GetSocketOption<Tcp> for MulticastEnableLoopback {
    fn get_sockopt(&mut self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => get_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, self),
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_LOOP, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Tcp> for MulticastEnableLoopback {
    fn set_sockopt(&self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => set_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, self),
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_LOOP, self),
            _ => None,
        }
    }
}

impl GetSocketOption<Tcp> for MulticastHops {
    fn get_sockopt(&mut self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => get_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_TTL, self),
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_HOPS, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Tcp> for MulticastHops {
    fn set_sockopt(&self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => set_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_TTL, self),
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_HOPS, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Tcp> for MulticastJoinGroup {
    fn set_sockopt(&self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        use super::options::Mreq;
        match (pro.family, &self.0) {
            (libc::AF_INET, Mreq::V4(ref mreq)) => set_sockopt(libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP, mreq),
            (libc::AF_INET6, Mreq::V6(ref mreq)) => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_JOIN_GROUP, mreq),
            _ => None,
        }
    }
}

impl SetSocketOption<Tcp> for MulticastLeaveGroup {
    fn set_sockopt(&self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        use super::options::Mreq;
        match (pro.family, &self.0) {
            (libc::AF_INET, Mreq::V4(ref mreq)) => set_sockopt(libc::IPPROTO_IP, libc::IP_DROP_MEMBERSHIP, mreq),
            (libc::AF_INET6, Mreq::V6(ref mreq)) => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_LEAVE_GROUP, mreq),
            _ => None,
        }
    }
}

impl GetSocketOption<Tcp> for NoDelay {
    fn get_sockopt(&mut self, _: &Tcp) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::IPPROTO_TCP, libc::TCP_NODELAY, self)
    }
}

impl SetSocketOption<Tcp> for NoDelay {
    fn set_sockopt(&self, _: &Tcp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        set_sockopt(libc::IPPROTO_TCP, libc::TCP_NODELAY, self)
    }
}

impl SetSocketOption<Tcp> for OutboundInterface {
    fn set_sockopt(&self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        use super::options::Interface;
        match (pro.family, &self.0) {
            (libc::AF_INET, Interface::V4(ref addr)) => set_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_IF, addr),
            (libc::AF_INET6, Interface::V6(ref scope_id)) => {
                set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_IF, scope_id)
            }
            _ => None,
        }
    }
}

impl GetSocketOption<Tcp> for UnicastHops {
    fn get_sockopt(&mut self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => get_sockopt(libc::IPPROTO_IP, libc::IP_TTL, self),
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Tcp> for UnicastHops {
    fn set_sockopt(&self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => set_sockopt(libc::IPPROTO_IP, libc::IP_TTL, self),
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS, self),
            _ => None,
        }
    }
}

impl GetSocketOption<Tcp> for V6Only {
    fn get_sockopt(&mut self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Tcp> for V6Only {
    fn set_sockopt(&self, pro: &Tcp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, self),
            _ => None,
        }
    }
}

#[test]
fn test_sockopt_v4() {
    use std::fmt;

    trait SocketOption<Tcp>: GetSocketOption<Tcp> + SetSocketOption<Tcp> + fmt::Debug {}

    impl<T> SocketOption<Tcp> for T where T: GetSocketOption<Tcp> + SetSocketOption<Tcp> + fmt::Debug {}

    let v4: Vec<Box<dyn SocketOption<Tcp>>> = vec![
        Box::new(MulticastEnableLoopback::new(false)),
        Box::new(MulticastHops::new(0)),
        Box::new(NoDelay::new(false)),
        Box::new(UnicastHops::new(0)),
    ];
    v4.into_iter().for_each(|mut x| {
        let tcp = Tcp::v4();
        let get = x.get_sockopt(&tcp).unwrap();
        let set = x.set_sockopt(&tcp).unwrap();
        println!("{:?}", x);
        assert_eq!(get.0, set.0);
        assert_eq!(get.1, set.1);
        assert_eq!(get.2, set.2 as _);
        assert_eq!(get.3, set.3);
    })
}

#[test]
fn test_sockopt_v6() {
    use std::fmt;

    trait SocketOption<Tcp>: GetSocketOption<Tcp> + SetSocketOption<Tcp> + fmt::Debug {}

    impl<T> SocketOption<Tcp> for T where T: GetSocketOption<Tcp> + SetSocketOption<Tcp> + fmt::Debug {}

    let v6: Vec<Box<dyn SocketOption<Tcp>>> = vec![
        Box::new(MulticastEnableLoopback::new(false)),
        Box::new(MulticastHops::new(0)),
        Box::new(NoDelay::new(false)),
        Box::new(UnicastHops::new(0)),
        Box::new(V6Only::new(false)),
    ];
    v6.into_iter().for_each(|mut x| {
        let tcp = Tcp::v6();
        let get = x.get_sockopt(&tcp).unwrap();
        let set = x.set_sockopt(&tcp).unwrap();
        println!("{:?}", x);
        assert_eq!(get.0, set.0);
        assert_eq!(get.1, set.1);
        assert_eq!(get.2, set.2 as _);
        assert_eq!(get.3, set.3);
    })
}

#[test]
fn test_resolver_unspec() {
    let ctx = &IoContext::new().unwrap();
    let res = TcpResolver::tcp(ctx);
    let mut it = res.resolve("localhost", 0).unwrap();
    let ep = it.next().unwrap();
    assert_eq!(ep.addr().is_loopback(), true);
}

#[test]
fn test_resolver_v4() {
    let ctx = &IoContext::new().unwrap();
    let res = TcpResolver::new(ctx, Tcp::v4());
    let mut it = res.resolve("localhost", 0).unwrap();
    let ep = it.next().unwrap();
    assert_eq!(ep.addr(), IpAddrV4::loopback());
}

#[test]
fn test_resolver_v6() {
    let ctx = &IoContext::new().unwrap();
    let res = TcpResolver::new(ctx, Tcp::v6());
    let mut it = res.resolve("localhost", 0).unwrap();
    let ep = it.next().unwrap();
    assert_eq!(ep.addr(), IpAddrV6::loopback());
}
