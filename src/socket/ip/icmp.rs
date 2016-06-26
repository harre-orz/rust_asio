use std::io;
use std::mem;
use std::cell::Cell;
use {IoObject, IoService, Strand, Cancel};
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
            unreachable!("Invalid domain ({}).", self.ss.ss_family);
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
    /// use asio::IoService;
    /// use asio::ip::{Icmp, IcmpSocket};
    ///
    /// let io = IoService::new();
    /// // Make a ICMP socket.
    /// let icmp = IcmpSocket::new(&io, Icmp::v4());
    ///
    /// // Make a ICMPv6 socket.
    /// let icmpv6 = IcmpSocket::new(&io, Icmp::v6());
    /// ```
    pub fn new(io: &IoService, pro: Icmp) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(IcmpSocket {
            actor: EpollIoActor::new(io, fd),
            nonblock: Cell::new(false),
        })
    }
}

impl IoObject for IcmpSocket {
    fn io_service(&self) -> &IoService {
        self.actor.io_service()
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
    fn cancel(&self) {
        cancel_io(self)
    }
}

impl SocketConnector for IcmpSocket {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()> {
        connect_syncd(self, ep)
    }

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        let soc = a(obj);
        connect_async(soc, ep, callback, obj)
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
        where A: FnOnce(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj.get_mut());
        recv_async(soc, buf, flags, callback, obj)
    }

    fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send_syncd(self, buf, flags)
    }

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj);
        send_async(soc, buf, flags, callback, obj)
    }
}

impl SendToRecvFrom for IcmpSocket {
    fn recv_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, Self::Endpoint)> {
        recvfrom_syncd(self, buf, flags, unsafe { mem::uninitialized() })
    }

    fn async_recv_from<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: FnOnce(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<(usize, Self::Endpoint)>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj.get_mut());
        recvfrom_async(soc, buf, flags, unsafe { mem::uninitialized() }, callback, obj)
    }

    fn send_to(&self, buf: &[u8], flags: i32, ep: &Self::Endpoint) -> io::Result<usize> {
        sendto_syncd(self, buf, flags, ep)
    }

    fn async_send_to<A, F, T>(a: A, flags: i32, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj);
        sendto_async(soc, buf, flags, ep, callback, obj)
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

impl Cancel for IcmpResolver {
    fn cancel(&self) {
        unimplemented!();
    }
}

impl Resolver for IcmpResolver {
    type Protocol = Icmp;

    fn resolve<'a, T: IoObject, Q: ResolveQuery<'a, Self>>(&self, io: &T, query: Q) -> io::Result<Q::Iter> {
        query.query(Icmp { family: AF_UNSPEC, protocol: 0 })
    }

    fn async_resolve<'a, Q, A, F, T>(a: A, query: Q, callback: F, obj: &Strand<T>)
        where Q: ResolveQuery<'a, Self> + 'static,
              A: FnOnce(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<Q::Iter>) + Send + 'static,
              T: 'static {
        unimplemented!();
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
    for e in re.resolve(&io, ("127.0.0.1", "")).unwrap() {
        assert!(e.endpoint() == IcmpEndpoint::new((IpAddrV4::new(127,0,0,1), 0)));
    }
}
