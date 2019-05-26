//

use super::{
    IpAddr, IpAddrV4, IpAddrV6, IpEndpoint, MulticastEnableLoopback, MulticastHops, MulticastJoinGroup,
    MulticastLeaveGroup, OutboundInterface, Resolver, ResolverIter, ResolverQuery, UnicastHops, V6Only,
};
use dgram_socket::DgramSocket;
use executor::IoContext;
use libc;
use socket_base::{get_sockopt, set_sockopt, GetSocketOption, Protocol, SetSocketOption};
use std::io;
use std::fmt;
use std::mem;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Udp {
    family: i32,
}

impl Udp {
    pub const fn v4() -> Self {
        Udp { family: libc::AF_INET }
    }

    pub const fn v6() -> Self {
        Udp { family: libc::AF_INET6 }
    }
}

impl Protocol for Udp {
    type Endpoint = IpEndpoint<Self>;
    type Socket = DgramSocket<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_DGRAM as i32
    }

    fn protocol_type(&self) -> i32 {
        libc::IPPROTO_UDP
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl From<Udp> for IpAddr {
    fn from(udp: Udp) -> Self {
        match udp.family {
            libc::AF_INET => IpAddr::V4(IpAddrV4::default()),
            libc::AF_INET6 => IpAddr::V6(IpAddrV6::default()),
            _ => unreachable!(),
        }
    }
}

impl Resolver<Udp> {
    pub fn udp(ctx: &IoContext) -> Self {
        Resolver::new(
            ctx,
            Udp {
                family: libc::AF_UNSPEC,
            },
        )
    }

    pub fn resolve<Q>(&self, host: Q, port: u16) -> io::Result<ResolverIter<Udp>>
    where
        Q: Into<ResolverQuery>,
    {
        self.addrinfo(host, port, 0)
    }
}

pub type UdpEndpoint = IpEndpoint<Udp>;
pub type UdpResolver = Resolver<Udp>;
pub type UdpSocket = DgramSocket<Udp>;

/// # Example
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt: MulticastEnableLoopback = soc.get_option().unwrap();
/// assert_eq!(opt.get(), true)
/// ```
impl GetSocketOption<Udp> for MulticastEnableLoopback {
    fn get_sockopt(&mut self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => get_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, self),
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_LOOP, self),
            _ => None,
        }
    }
}

/// # Example
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// soc.set_option(MulticastEnableLoopback::new(true)).unwrap();
/// ```
///
impl SetSocketOption<Udp> for MulticastEnableLoopback {
    fn set_sockopt(&self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => set_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, self),
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_LOOP, self),
            _ => None,
        }
    }
}

impl GetSocketOption<Udp> for MulticastHops {
    fn get_sockopt(&mut self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => get_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_TTL, self),
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_HOPS, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Udp> for MulticastHops {
    fn set_sockopt(&self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => set_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_TTL, self),
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_HOPS, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Udp> for MulticastJoinGroup {
    fn set_sockopt(&self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        use super::options::Mreq;
        match (pro.family, &self.0) {
            (libc::AF_INET, Mreq::V4(ref mreq)) => set_sockopt(libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP, mreq),
            (libc::AF_INET6, Mreq::V6(ref mreq)) => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_JOIN_GROUP, mreq),
            _ => None,
        }
    }
}

impl SetSocketOption<Udp> for MulticastLeaveGroup {
    fn set_sockopt(&self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        use super::options::Mreq;
        match (pro.family, &self.0) {
            (libc::AF_INET, Mreq::V4(ref mreq)) => set_sockopt(libc::IPPROTO_IP, libc::IP_DROP_MEMBERSHIP, mreq),
            (libc::AF_INET6, Mreq::V6(ref mreq)) => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_LEAVE_GROUP, mreq),
            _ => None,
        }
    }
}

impl SetSocketOption<Udp> for OutboundInterface {
    fn set_sockopt(&self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
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

impl GetSocketOption<Udp> for UnicastHops {
    fn get_sockopt(&mut self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => get_sockopt(libc::IPPROTO_IP, libc::IP_TTL, self),
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Udp> for UnicastHops {
    fn set_sockopt(&self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => set_sockopt(libc::IPPROTO_IP, libc::IP_TTL, self),
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS, self),
            _ => None,
        }
    }
}

impl GetSocketOption<Udp> for V6Only {
    fn get_sockopt(&mut self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Udp> for V6Only {
    fn set_sockopt(&self, pro: &Udp) -> Option<(libc::c_int, libc::c_int, *const libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, self),
            _ => None,
        }
    }
}

#[test]
fn test_resolver_unspec() {
    let ctx = &IoContext::new().unwrap();
    let res = UdpResolver::udp(ctx);
    let mut it = res.resolve("localhost", 0).unwrap();
    let ep = it.next().unwrap();
    assert_eq!(ep.addr().is_loopback(), true);
}

#[test]
fn test_resolver_v4() {
    let ctx = &IoContext::new().unwrap();
    let res = UdpResolver::new(ctx, Udp::v4());
    let mut it = res.resolve("localhost", 0).unwrap();
    let ep = it.next().unwrap();
    assert_eq!(ep.addr(), IpAddrV4::loopback());
}

#[test]
fn test_resolver_v6() {
    let ctx = &IoContext::new().unwrap();
    let res = UdpResolver::new(ctx, Udp::v6());
    let mut it = res.resolve("localhost", 0).unwrap();
    let ep = it.next().unwrap();
    assert_eq!(ep.addr(), IpAddrV6::loopback());
}
