//

use super::{
    IpAddr, IpAddrV4, IpAddrV6, IpEndpoint, MulticastEnableLoopback, MulticastHops,
    MulticastJoinGroup, MulticastLeaveGroup, OutboundInterface, Resolver, ResolverIter,
    ResolverQuery, UnicastHops, V6Only,
};
use dgram_socket::DgramSocket;
use executor::IoContext;
use libc;
use socket_base::{get_sockopt, set_sockopt, GetSocketOption, Protocol, SetSocketOption};
use std::io;
use std::mem::MaybeUninit;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Icmp {
    family: i32,
    protocol: i32,
}

impl Icmp {
    pub const fn v4() -> Self {
        Icmp {
            family: libc::AF_INET,
            protocol: libc::IPPROTO_ICMP,
        }
    }

    pub const fn v6() -> Self {
        Icmp {
            family: libc::AF_INET6,
            protocol: libc::IPPROTO_ICMPV6,
        }
    }
}

impl Protocol for Icmp {
    type Endpoint = IpEndpoint<Self>;
    type Socket = DgramSocket<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_RAW as i32
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    fn uninit(&self) -> MaybeUninit<Self::Endpoint> {
        MaybeUninit::uninit()
    }
}

impl From<Icmp> for IpAddr {
    fn from(icmp: Icmp) -> Self {
        match icmp.family {
            libc::AF_INET => IpAddr::V4(IpAddrV4::default()),
            libc::AF_INET6 => IpAddr::V6(IpAddrV6::default()),
            _ => unreachable!(),
        }
    }
}

impl Resolver<Icmp> {
    pub fn icmp(ctx: &IoContext) -> Self {
        Resolver::new(
            ctx,
            Icmp {
                family: libc::AF_UNSPEC,
                protocol: 0,
            },
        )
    }

    pub fn resolve<Q>(&self, host: Q) -> io::Result<ResolverIter<Icmp>>
    where
        Q: Into<ResolverQuery>,
    {
        self.addrinfo(host, 0, 0)
    }
}

pub type IcmpEndpoint = IpEndpoint<Icmp>;
pub type IcmpResolver = Resolver<Icmp>;
pub type IcmpSocket = DgramSocket<Icmp>;

impl GetSocketOption<Icmp> for MulticastEnableLoopback {
    fn get_sockopt(
        &mut self,
        pro: &Icmp,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => get_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, self),
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_LOOP, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Icmp> for MulticastEnableLoopback {
    fn set_sockopt(
        &self,
        pro: &Icmp,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        match pro.family {
            libc::AF_INET => set_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, self),
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_LOOP, self),
            _ => None,
        }
    }
}

impl GetSocketOption<Icmp> for MulticastHops {
    fn get_sockopt(
        &mut self,
        pro: &Icmp,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => get_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_TTL, self),
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_HOPS, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Icmp> for MulticastHops {
    fn set_sockopt(
        &self,
        pro: &Icmp,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        match pro.family {
            libc::AF_INET => set_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_TTL, self),
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_HOPS, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Icmp> for MulticastJoinGroup {
    fn set_sockopt(
        &self,
        pro: &Icmp,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        use super::options::Mreq;
        match (pro.family, &self.0) {
            (libc::AF_INET, Mreq::V4(ref mreq)) => {
                set_sockopt(libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP, mreq)
            }
            (libc::AF_INET6, Mreq::V6(ref mreq)) => {
                set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_JOIN_GROUP, mreq)
            }
            _ => None,
        }
    }
}

impl SetSocketOption<Icmp> for MulticastLeaveGroup {
    fn set_sockopt(
        &self,
        pro: &Icmp,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        use super::options::Mreq;
        match (pro.family, &self.0) {
            (libc::AF_INET, Mreq::V4(ref mreq)) => {
                set_sockopt(libc::IPPROTO_IP, libc::IP_DROP_MEMBERSHIP, mreq)
            }
            (libc::AF_INET6, Mreq::V6(ref mreq)) => {
                set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_LEAVE_GROUP, mreq)
            }
            _ => None,
        }
    }
}

impl SetSocketOption<Icmp> for OutboundInterface {
    fn set_sockopt(
        &self,
        pro: &Icmp,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        use super::options::Interface;
        match (pro.family, &self.0) {
            (libc::AF_INET, Interface::V4(ref addr)) => {
                set_sockopt(libc::IPPROTO_IP, libc::IP_MULTICAST_IF, addr)
            }
            (libc::AF_INET6, Interface::V6(ref scope_id)) => {
                set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_IF, scope_id)
            }
            _ => None,
        }
    }
}

impl GetSocketOption<Icmp> for UnicastHops {
    fn get_sockopt(
        &mut self,
        pro: &Icmp,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET => get_sockopt(libc::IPPROTO_IP, libc::IP_TTL, self),
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Icmp> for UnicastHops {
    fn set_sockopt(
        &self,
        pro: &Icmp,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        match pro.family {
            libc::AF_INET => set_sockopt(libc::IPPROTO_IP, libc::IP_TTL, self),
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS, self),
            _ => None,
        }
    }
}

impl GetSocketOption<Icmp> for V6Only {
    fn get_sockopt(
        &mut self,
        pro: &Icmp,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        match pro.family {
            libc::AF_INET6 => get_sockopt(libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, self),
            _ => None,
        }
    }
}

impl SetSocketOption<Icmp> for V6Only {
    fn set_sockopt(
        &self,
        pro: &Icmp,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        match pro.family {
            libc::AF_INET6 => set_sockopt(libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, self),
            _ => None,
        }
    }
}

#[test]
fn test_resolver_unspec() {
    let ctx = &IoContext::new().unwrap();
    let res = IcmpResolver::icmp(ctx);
    let mut it = res.resolve("localhost").unwrap();
    let ep = it.next().unwrap();
    assert_eq!(ep.addr().is_loopback(), true);
}

#[test]
fn test_resolver_v4() {
    let ctx = &IoContext::new().unwrap();
    let res = IcmpResolver::new(ctx, Icmp::v4());
    let mut it = res.resolve("localhost").unwrap();
    let ep = it.next().unwrap();
    assert_eq!(ep.addr(), IpAddrV4::loopback());
}

#[test]
fn test_resolver_v6() {
    let ctx = &IoContext::new().unwrap();
    let res = IcmpResolver::new(ctx, Icmp::v6());
    let mut it = res.resolve("localhost").unwrap();
    let ep = it.next().unwrap();
    assert_eq!(ep.addr(), IpAddrV6::loopback());
}
