use ffi::*;
use prelude::{Endpoint, Protocol};
use socket_base::{Tx, Rx};
use socket_builder::SocketBuilder;
use dgram_socket::DgramSocket;
use ip::{IpProtocol, IpEndpoint, Resolver, ResolverIter, ResolverQuery};

use std::io;
use std::fmt;
use std::mem;
use std::marker::PhantomData;

/// The Internet Control Message Protocol.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Icmp {
    family: i32,
    protocol: i32,
}

impl Protocol for Icmp {
    type Endpoint = IpEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_RAW as i32
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl IpProtocol for Icmp {
    /// Represents a ICMP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{IpProtocol, IpAddrV4, Icmp, IcmpEndpoint};
    ///
    /// let ep = IcmpEndpoint::new(IpAddrV4::any(), 0);
    /// assert_eq!(Icmp::v4(), ep.protocol());
    /// ```
    fn v4() -> Icmp {
        Icmp { family: AF_INET as i32, protocol: IPPROTO_ICMP }
    }

    /// Represents a ICMPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{IpProtocol, IpAddrV6, Icmp, IcmpEndpoint};
    ///
    /// let ep = IcmpEndpoint::new(IpAddrV6::any(), 0);
    /// assert_eq!(Icmp::v6(), ep.protocol());
    /// ```
    fn v6() -> Icmp {
        Icmp { family: AF_INET6 as i32, protocol: IPPROTO_ICMPV6 }
    }

    fn from_ai(ai: *mut addrinfo) -> Option<Self::Endpoint> {
        if ai.is_null() {
            return None
        }

        unsafe {
            let ai = &*ai;
            let mut ep = IpEndpoint {
                ss: mem::transmute_copy(&*(ai.ai_addr as *const SockAddr<sockaddr_storage>)),
                _marker: PhantomData,
            };
            ep.resize(ai.ai_addrlen);
            Some(ep)
        }
    }
}

impl fmt::Debug for Icmp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.family_type() {
            AF_INET => write!(f, "ICMPv4"),
            AF_INET6 => write!(f, "ICMPv6"),
            _ => unreachable!("Invalid address family ({}).", self.family),
        }
    }
}

impl Endpoint<Icmp> for IpEndpoint<Icmp> {
    fn protocol(&self) -> Icmp {
        let family_type = self.ss.sa.ss_family as i32;
        match family_type {
            AF_INET => Icmp::v4(),
            AF_INET6 =>  Icmp::v6(),
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


impl<'a> ResolverQuery<Icmp> for &'a str {
    fn iter(self) -> io::Result<ResolverIter<Icmp>> {
        ResolverIter::new(&Icmp { family: AF_UNSPEC, protocol: 0 }, self.as_ref(), "", 0)
    }
}

/// The ICMP endpoint type.
pub type IcmpEndpoint = IpEndpoint<Icmp>;

/// The ICMP socket type.
pub type IcmpTxSocket = DgramSocket<Icmp, Tx>;

pub type IcmpRxSocket = DgramSocket<Icmp, Rx>;

pub type IcmpBuilder = SocketBuilder<Icmp, DgramSocket<Icmp, Tx>, DgramSocket<Icmp, Rx>>;

/// The ICMP resolver type.
pub type IcmpResolver = Resolver<Icmp, DgramSocket<Icmp, Tx>, DgramSocket<Icmp, Rx>>;

#[test]
fn test_icmp() {
    assert!(Icmp::v4() == Icmp::v4());
    assert!(Icmp::v6() == Icmp::v6());
    assert!(Icmp::v4() != Icmp::v6());
}

#[test]
fn test_icmp_resolve() {
    use core::IoContext;
    use ip::*;

    let ctx = &IoContext::new().unwrap();
    let re = IcmpResolver::new(ctx);
    for ep in re.resolve("127.0.0.1").unwrap() {
        assert!(ep == IcmpEndpoint::new(IpAddrV4::loopback(), 0));
    }
    for ep in re.resolve("::1").unwrap() {
        assert!(ep == IcmpEndpoint::new(IpAddrV6::loopback(), 0));
    }
    for ep in re.resolve(("localhost")).unwrap() {
        assert!(ep.addr().is_loopback());
    }
}

#[test]
#[ignore]
fn test_format() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    println!("{:?}", Icmp::v4());
    println!("{:?}", IcmpEndpoint::new(Icmp::v4(), 12345));
    println!("{:?}", IcmpSocket::new(ctx, Icmp::v4()).unwrap());
    println!("{:?}", IcmpResolver::new(ctx));
}
