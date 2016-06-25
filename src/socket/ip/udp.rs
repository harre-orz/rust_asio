use std::io;
use std::mem;
use std::cell::Cell;
use {IoObject, Strand, Cancel};
use backbone::EpollIoActor;
use socket::*;
use socket::ip::*;
use ops::*;
use ops::async::*;


/// Encapsulates the flags needed for UDP.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Udp {
    family: i32,
}

impl Udp {

    /// Make the UDP for IPv4.
    ///
    /// # Example
    /// ```
    /// use asio::ip::Udp;
    /// let pro = Udp::v4();
    /// ```
    pub fn v4() -> Udp {
        Udp { family: AF_INET as i32 }
    }

    /// Make the UDP for IPv6.
    ///
    /// # Example
    /// ```
    /// use asio::ip::Udp;
    /// let pro = Udp::v6();
    /// ```
    pub fn v6() -> Udp {
        Udp { family: AF_INET6 as i32 }
    }
}

impl Protocol for Udp {
    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM as i32
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl Endpoint<Udp> for IpEndpoint<Udp> {
    fn protocol(&self) -> Udp {
        if self.is_v4() {
            Udp::v4()
        } else if self.is_v6() {
            Udp::v6()
        } else {
            unreachable!("Invalid domain ({}).", self.ss.ss_family);
        }
    }
}

/// The type of a UDP endpoint.
pub type UdpEndpoint = IpEndpoint<Udp>;

/// The UDP socket type.
pub struct UdpSocket {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl UdpSocket {
    /// Make a UDP socket.
    ///
    /// # Example
    /// ```
    /// use asio::ip::{Udp, UdpSocket};
    ///
    /// // Make a UDP socket for IPv4.
    /// let udp4 = UdpSocket::new(Udp::v4()).unwrap();
    ///
    /// // Make a UDP socket for IPv6.
    /// let udp6 = UdpSocket::new(Udp::v6()).unwrap();
    /// ```
    pub fn new(pro: Udp) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(UdpSocket {
            actor: EpollIoActor::new(fd),
            nonblock: Cell::new(false),
        })
    }
}

impl AsRawFd for UdpSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for UdpSocket {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl NonBlocking for UdpSocket {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl Socket for UdpSocket {
    type Protocol = Udp;
    type Endpoint = UdpEndpoint;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl IpSocket for UdpSocket {
}

impl Cancel for UdpSocket {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self {
        cancel_io(a(obj), obj)
    }
}

impl SocketConnector for UdpSocket {
    fn connect<T: IoObject>(&self, io: &T, ep: &Self::Endpoint) -> io::Result<()> {
        connect_syncd(self, ep, io.io_service())
    }

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static
    {
        let soc = a(obj);
        connect_async(soc, ep, callback, obj)
    }

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint> {
        getpeername(self, unsafe { mem::uninitialized() })
    }
}

impl SendRecv for UdpSocket {
    fn recv<T: IoObject>(&self, io: &T, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv_syncd(self, buf, flags, io.io_service())
    }

    fn async_recv<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: FnOnce(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj.get_mut());
        recv_async(soc, buf, flags, callback, obj)
    }

    fn send<T: IoObject>(&self, io: &T, buf: &[u8], flags: i32) -> io::Result<usize> {
        send_syncd(self, buf, flags, io.io_service())
    }

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj);
        send_async(soc, buf, flags, callback, obj)
    }
}

impl SendToRecvFrom for UdpSocket {
    fn recv_from<T: IoObject>(&self, io: &T, buf: &mut [u8], flags: i32) -> io::Result<(usize, Self::Endpoint)> {
        recvfrom_syncd(self, buf, flags, unsafe { mem::uninitialized() }, io.io_service())
    }

    fn async_recv_from<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: FnOnce(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<(usize, Self::Endpoint)>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj.get_mut());
        recvfrom_async(soc, buf, flags, unsafe { mem::uninitialized() }, callback, obj)
    }

    fn send_to<T: IoObject>(&self, io: &T, buf: &[u8], flags: i32, ep: &Self::Endpoint) -> io::Result<usize> {
        sendto_syncd(self, buf, flags, ep, io.io_service())
    }

    fn async_send_to<A, F, T>(a: A, flags: i32, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj);
        sendto_async(soc, buf, flags, ep, callback, obj)
    }
}

impl DgramSocket for UdpSocket {
}

/// The UDP resolver type.
pub struct UdpResolver {
}

impl UdpResolver {
    /// Make a UDP resolver.
    pub fn new() -> Self {
        UdpResolver {
        }
    }
}

impl Cancel for UdpResolver {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self {
        unimplemented!();
    }
}

impl Resolver for UdpResolver {
    type Protocol = Udp;

    fn resolve<'a, T: IoObject, Q: ResolveQuery<'a, Self>>(&self, io: &T, query: Q) -> io::Result<Q::Iter> {
        query.query(Udp { family: AF_UNSPEC })
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
fn test_udp() {
    assert!(Udp::v4() == Udp::v4());
    assert!(Udp::v6() == Udp::v6());
    assert!(Udp::v4() != Udp::v6());
}

#[test]
fn test_udp_resolve() {
    use IoService;
    use super::IpAddrV4;

    let io = IoService::new();
    let re = UdpResolver::new();
    for e in re.resolve(&io, ("127.0.0.1", "80")).unwrap() {
        assert!(e.endpoint() == UdpEndpoint::new((IpAddrV4::new(127,0,0,1), 80)));
    }
}
