use std::io;
use std::mem;
use std::cell::Cell;
use {Strand, Cancel};
use backbone::EpollIoActor;
use socket::*;
use socket::ip::*;
use ops::*;
use ops::async::*;

/// Encapsulates the flags needed for ICMP(v6).
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Icmp {
    family: i32,
    protocol: i32,
}

impl Icmp {
    /// Make the ICMP.
    ///
    /// # Example
    /// ```
    /// use asio::ip::Icmp;
    /// let pro = Icmp::v4();
    /// ```
    pub fn v4() -> Icmp {
        Icmp { family: AF_INET as i32, protocol: IPPROTO_ICMP }
    }

    /// Make the ICMPv6.
    ///
    /// # Example
    /// ```
    /// use asio::ip::Icmp;
    /// let pro = Icmp::v6();
    /// ```
    pub fn v6() -> Icmp {
        Icmp { family: AF_INET6 as i32, protocol: IPPROTO_ICMPV6 }
    }
}

impl Protocol for Icmp {
    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_RAW as i32
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }
}

impl Endpoint<Icmp> for IpEndpoint<Icmp> {
    fn protocol(&self) -> Icmp {
        if self.is_v4() {
            Icmp::v4()
        } else if self.is_v6() {
            Icmp::v6()
        } else {
            unreachable!("Invalid family code ({}).", self.ss.ss_family);
        }
    }
}

/// The type of a ICMP(v6) endpoint.
pub type IcmpEndpoint = IpEndpoint<Icmp>;

/// The ICMP(v6) socket type.
pub struct IcmpSocket {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl IcmpSocket {
    /// Make a ICMP(v6) socket.
    ///
    /// # Example
    /// ```
    /// use asio::ip::{Icmp, IcmpSocket};
    ///
    /// // Make a ICMP socket.
    /// let icmp = IcmpSocket::new(Icmp::v4());
    ///
    /// // Make a ICMPv6 socket.
    /// let icmpv6 = IcmpSocket::new(Icmp::v6());
    /// ```
    pub fn new(pro: Icmp) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(IcmpSocket {
            actor: EpollIoActor::new(fd),
            nonblock: Cell::new(false),
        })
    }
}

impl AsRawFd for IcmpSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for IcmpSocket {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl NonBlocking for IcmpSocket {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl Socket for IcmpSocket {
    type Protocol = Icmp;
    type Endpoint = IcmpEndpoint;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl IpSocket for IcmpSocket {
}

impl Cancel for IcmpSocket {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + 'static,
              T: 'static {
        cancel_io(a, obj)
    }
}

impl SocketConnector for IcmpSocket {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()> {
        connect_syncd(self, ep)
    }

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        connect_async(a, ep, callback, obj)
    }

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint> {
        getpeername(self, unsafe { mem::uninitialized() })
    }
}

impl SendRecv for IcmpSocket {
    fn recv(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv_syncd(self, buf, flags)
    }

    fn async_recv<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        recv_async(a, flags, callback, obj)
    }

    fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send_syncd(self, buf, flags)
    }

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        send_async(a, flags, callback, obj)
    }
}

impl SendToRecvFrom for IcmpSocket {
    fn recv_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, Self::Endpoint)> {
        recvfrom_syncd(self, buf, flags, unsafe { mem::uninitialized() })
    }

    fn async_recv_from<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<(usize, Self::Endpoint)>) + Send + 'static,
              T: 'static {
        recvfrom_async(a, flags, unsafe { mem::uninitialized() }, callback, obj)
    }

    fn send_to(&self, buf: &[u8], flags: i32, ep: &Self::Endpoint) -> io::Result<usize> {
        sendto_syncd(self, buf, flags, ep)
    }

    fn async_send_to<A, F, T>(a: A, flags: i32, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        sendto_async(a, flags, ep, callback, obj)
    }
}

impl RawSocket for IcmpSocket {
}

/// The ICMP(v6) resolver type.
pub struct IcmpResolver {
}

impl IcmpResolver {
    /// Make a ICMP(v6) resolver.
    pub fn new() -> Self {
        IcmpResolver {
        }
    }
}

impl Resolver for IcmpResolver {
    type Protocol = Icmp;

    fn resolve<'a, Q: ResolveQuery<'a, Self>>(&self, query: Q) -> io::Result<Q::Iter> {
        query.query(Icmp { family: AF_UNSPEC, protocol: 0 })
    }

    fn async_resolve<'a, Q, A, F, T>(a: A, query: Q, callback: F, obj: &Strand<T>)
        where Q: ResolveQuery<'a, Self> + 'static,
              A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<Q::Iter>) + Send + 'static,
              T: 'static {
        async_resolve(a, move || { query.query(Icmp { family: AF_UNSPEC, protocol: 0 }) }, callback, obj);
    }
}

#[test]
fn test_icmp() {
    assert!(Icmp::v4() == Icmp::v4());
    assert!(Icmp::v6() == Icmp::v6());
    assert!(Icmp::v4() != Icmp::v6());
}

#[test]
fn test_icmp_resolve() {
    use IoService;
    use super::IpAddrV4;

    let io = IoService::new();
    let re = IcmpResolver::new();
    for e in re.resolve(("127.0.0.1", "")).unwrap() {
        assert!(e.endpoint() == IcmpEndpoint::new((IpAddrV4::new(127,0,0,1), 0)));
    }
}
