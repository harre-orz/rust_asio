use crate::ip::{IpProtocol, Tcp};
use crate::sockaddr::AddressFamily;
use crate::socket_base::{GetSockOpt, Protocol, SetSockOpt};
use std::mem::MaybeUninit;
use std::num::NonZeroU8;
use std::{ptr, slice};

pub struct NoDelay(libc::c_int);

impl NoDelay {
    pub const ON: Self = Self(1);
    pub const OFF: Self = Self(0);

    pub const fn new(on: bool) -> Self {
        Self(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl SetSockOpt<Tcp> for NoDelay {
    fn data(&self, _: Tcp) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.0).cast(), size_of_val(&self.0)) }
    }
}

impl GetSockOpt<Tcp> for NoDelay {
    fn init(uninit: MaybeUninit<Self>, _: usize, _: Tcp) -> Self {
        unsafe { uninit.assume_init() }
    }
}

pub struct V6Only(libc::c_int);

impl V6Only {
    pub const ON: Self = Self(1);
    pub const OFF: Self = Self(0);

    pub const fn new(on: bool) -> Self {
        Self(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl<P> SetSockOpt<P> for V6Only
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, _: P) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.0).cast(), size_of_val(&self.0)) }
    }
}

impl<P> GetSockOpt<P> for V6Only
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(uninit: MaybeUninit<Self>, _: usize, _: P) -> Self {
        unsafe { uninit.assume_init() }
    }
}

pub struct McastLoop(libc::c_int);

impl McastLoop {
    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(on as libc::c_int)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl<P> SetSockOpt<P> for McastLoop
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, _: P) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.0).cast(), size_of_val(&self.0)) }
    }
}

impl<P> GetSockOpt<P> for McastLoop
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(uninit: MaybeUninit<Self>, _: usize, _: P) -> Self {
        unsafe { uninit.assume_init() }
    }
}

pub struct UcastHops(libc::c_int);

impl UcastHops {
    pub const fn new(count: NonZeroU8) -> Self {
        Self(count.get() as libc::c_int)
    }

    pub const fn get(&self) -> u8 {
        self.0 as u8
    }
}

impl<P> SetSockOpt<P> for UcastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, _: P) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.0).cast(), size_of_val(&self.0)) }
    }
}

impl<P> GetSockOpt<P> for UcastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(uninit: MaybeUninit<Self>, _: usize, _: P) -> Self {
        unsafe { uninit.assume_init() }
    }
}

enum McastMember {
    V4(libc::ip_mreqn),
    V6(libc::ipv6_mreq),
}

pub struct McastJoin(McastMember);

impl McastJoin {
    pub fn v4() -> Self {
        Self(McastMember::V4(libc::ip_mreqn {
            imr_multiaddr: libc::in_addr { s_addr: 0 },
            imr_address: libc::in_addr { s_addr: 0 },
            imr_ifindex: 0,
        }))
    }

    pub fn v6() -> Self {
        Self(McastMember::V6(libc::ipv6_mreq {
            ipv6mr_multiaddr: libc::in6_addr { s6_addr: [0; 16] },
            ipv6mr_interface: 0,
        }))
    }
}

impl<P> SetSockOpt<P> for McastJoin
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> &[u8] {
        unsafe {
            match (pro.family_type(), &self.0) {
                (AddressFamily::AF_INET, McastMember::V4(imr)) => {
                    slice::from_raw_parts(ptr::from_ref(imr).cast(), size_of_val(imr))
                }
                (AddressFamily::AF_INET6, McastMember::V6(ipv6mr)) => {
                    slice::from_raw_parts(ptr::from_ref(ipv6mr).cast(), size_of_val(ipv6mr))
                }
                _ => &[],
            }
        }
    }
}

pub struct McastLeave(McastMember);

impl McastLeave {
    pub fn v4() -> Self {
        Self(McastMember::V4(libc::ip_mreqn {
            imr_multiaddr: libc::in_addr { s_addr: 0 },
            imr_address: libc::in_addr { s_addr: 0 },
            imr_ifindex: 0,
        }))
    }

    pub fn v6() -> Self {
        Self(McastMember::V6(libc::ipv6_mreq {
            ipv6mr_multiaddr: libc::in6_addr { s6_addr: [0; 16] },
            ipv6mr_interface: 0,
        }))
    }
}

impl<P> SetSockOpt<P> for McastLeave
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> &[u8] {
        unsafe {
            match (pro.family_type(), &self.0) {
                (AddressFamily::AF_INET, McastMember::V4(imr)) => {
                    slice::from_raw_parts(ptr::from_ref(imr).cast(), size_of_val(imr))
                }
                (AddressFamily::AF_INET6, McastMember::V6(ipv6mr)) => {
                    slice::from_raw_parts(ptr::from_ref(ipv6mr).cast(), size_of_val(ipv6mr))
                }
                _ => &[],
            }
        }
    }
}

#[cfg(unix)]
mod ffi {
    use super::*;
    use crate::ip::endpoint::Ip;
    use crate::socket_base::SockOpt;

    impl SockOpt<Tcp> for NoDelay {
        fn key(_: Tcp) -> (i32, i32) {
            (libc::IPPROTO_TCP, libc::TCP_NODELAY)
        }
    }

    impl<P> SockOpt<P> for V6Only
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn key(pro: P) -> (i32, i32) {
            match IpProtocol::version(pro) {
                Ip::V4 => Default::default(),
                Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_V6ONLY),
            }
        }
    }

    impl<P> SockOpt<P> for UcastHops
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn key(pro: P) -> (libc::c_int, libc::c_int) {
            match IpProtocol::version(pro) {
                Ip::V4 => (libc::IPPROTO_IP, libc::IP_TTL),
                Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS),
            }
        }
    }

    impl<P> SockOpt<P> for McastLoop
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn key(pro: P) -> (libc::c_int, libc::c_int) {
            match IpProtocol::version(pro) {
                Ip::V4 => (libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP),
                Ip::V6 => (0, 0),
            }
        }
    }

    impl<P> SockOpt<P> for McastJoin
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn key(pro: P) -> (libc::c_int, libc::c_int) {
            match IpProtocol::version(pro) {
                Ip::V4 => (libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP),
                Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_ADD_MEMBERSHIP),
            }
        }
    }

    impl<P> SockOpt<P> for McastLeave
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn key(pro: P) -> (libc::c_int, libc::c_int) {
            match IpProtocol::version(pro) {
                Ip::V4 => (libc::IPPROTO_IP, libc::IP_DROP_MEMBERSHIP),
                Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_DROP_MEMBERSHIP),
            }
        }
    }
}
