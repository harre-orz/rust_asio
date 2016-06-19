use std::io;
use std::mem;
use std::cell::Cell;
use {Strand, Cancel};
use backbone::EpollIoActor;
use socket::*;
use socket::ip::*;
use ops::*;
use ops::async::*;

/// Encapsulates the flags needed for TCP.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Tcp {
    family: i32,
}

impl Tcp {
    /// Make the TCP for IPv4.
    ///
    /// # Example
    /// ```
    /// use asio::ip::Tcp;
    /// let pro = Tcp::v4();
    /// ```
    pub fn v4() -> Tcp {
        Tcp { family: AF_INET as i32 }
    }

    /// Make the TCP for IPv6.
    ///
    /// # Example
    /// ```
    /// use asio::ip::Tcp;
    /// let pro = Tcp::v6();
    /// ```
    pub fn v6() -> Tcp {
        Tcp { family: AF_INET6 as i32 }
    }
}

impl Protocol for Tcp {
    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM as i32
    }

    fn protocol_type(&self) -> i32 {
        IPPROTO_TCP as i32
    }
}

impl Endpoint<Tcp> for IpEndpoint<Tcp> {
    fn protocol(&self) -> Tcp {
        if self.is_v4() {
            Tcp::v4()
        } else if self.is_v6() {
            Tcp::v6()
        } else {
            unreachable!("Invalid family code ({}).", self.ss.ss_family);
        }
    }
}

/// The type of a TCP endpoint.
pub type TcpEndpoint = IpEndpoint<Tcp>;

/// The TCP socket type.
pub struct TcpSocket {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl TcpSocket {
    /// Make the TCP socket.
    ///
    /// # Example
    /// ```
    /// use asio::ip::{Tcp, TcpSocket};
    ///
    /// // Make a TCP socket for IPv4.
    /// let tcp4 = TcpSocket::new(Tcp::v4()).unwrap();
    ///
    /// // Make a TCP socket for IPv6.
    /// let tcp6 = TcpSocket::new(Tcp::v6()).unwrap();
    /// ```
    pub fn new(pro: Tcp) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(TcpSocket {
            actor: EpollIoActor::new(fd),
            nonblock: Cell::new(false),
        })
    }
}

impl AsRawFd for TcpSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for TcpSocket {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl NonBlocking for TcpSocket {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl Socket for TcpSocket {
    type Protocol = Tcp;
    type Endpoint = TcpEndpoint;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl IpSocket for TcpSocket {
}

impl Cancel for TcpSocket {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + 'static,
              T: 'static {
        cancel_io(a, obj)
    }
}

impl SocketConnector for TcpSocket {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()> {
        connect_syncd(self, ep)
    }

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        connect_async(a, ep, move|obj,res| {
            callback(obj,res);
        }, obj)
    }

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint> {
        getpeername(self, unsafe { mem::uninitialized() })
    }
}

impl SendRecv for TcpSocket {
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

impl ReadWrite for TcpSocket {
    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        recv_syncd(self, buf, 0)
    }

    fn async_read_some<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        recv_async(a, 0, callback, obj)
    }

    fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        send_syncd(self, buf, 0)
    }

    fn async_write_some<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        send_async(a, 0, callback, obj)
    }

    fn async_read_until<A, C, F, T>(a: A, cond: C, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut StreamBuf) + Send + 'static,
              C: MatchCondition + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        read_until_async(a, cond, callback, obj, 0);
    }
}

impl StreamSocket for TcpSocket {
}

/// The TCP listener type.
pub struct TcpListener {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl TcpListener {
    /// Make the TCP listener.
    ///
    /// # Example
    /// ```
    /// use asio::ip::{Tcp, TcpListener};
    ///
    /// // Make a TcpListener for IPv4.
    /// let tcp4 = TcpListener::new(Tcp::v4()).unwrap();
    ///
    /// // Make a TcpListener for IPv6.
    /// let tcp6 = TcpListener::new(Tcp::v6()).unwrap();
    /// ```
    pub fn new(pro: Tcp) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(TcpListener {
            actor: EpollIoActor::new(fd),
            nonblock: Cell::new(false),
        })
    }
}

impl AsRawFd for TcpListener {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for TcpListener {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl NonBlocking for TcpListener {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl Socket for TcpListener {
    type Protocol = Tcp;
    type Endpoint = TcpEndpoint;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl IpSocket for TcpListener {
}

impl Cancel for TcpListener {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + 'static,
              T: 'static {
        cancel_io(a, obj)
    }
}

impl SocketListener for TcpListener {
    type Socket = TcpSocket;

    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)> {
        let (fd, ep) = try!(accept_syncd(self, unsafe { mem::uninitialized() }));
        Ok((TcpSocket {
            actor: EpollIoActor::new(fd),
            nonblock: Cell::new(false),
        }, ep))
    }

    fn async_accept<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<(Self::Socket, Self::Endpoint)>) + Send + 'static,
              T: 'static {
        accept_async(a, unsafe { mem::uninitialized() },
                     move |obj, res| {
                         match res {
                             Ok((fd, ep)) =>
                                 callback(obj, Ok((TcpSocket {
                                     actor: EpollIoActor::new(fd),
                                     nonblock: Cell::new(false),
                                 }, ep))),
                             Err(err) => callback(obj, Err(err)),
                         }
                     }, obj);
    }
}

/// The TCP resolver type.
pub struct TcpResolver {
}

impl TcpResolver{
    /// Make the TCP resolver.
    pub fn new() -> Self {
        TcpResolver {
        }
    }
}

impl Resolver for TcpResolver {
    type Protocol = Tcp;

    fn resolve<'a, Q: ResolveQuery<'a, Self>>(&self, query: Q) -> io::Result<Q::Iter> {
        query.query(Tcp { family: AF_UNSPEC })
    }

    fn async_resolve<'a, Q, A, F, T>(a: A, query: Q, callback: F, obj: &Strand<T>)
        where Q: ResolveQuery<'a, Self> + 'static,
              A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<Q::Iter>) + Send + 'static,
              T: 'static {
        async_resolve(a, move || { query.query(Tcp { family: AF_UNSPEC }) }, callback, obj);
    }
}

#[test]
fn test_tcp() {
    assert!(Tcp::v4() == Tcp::v4());
    assert!(Tcp::v6() == Tcp::v6());
    assert!(Tcp::v4() != Tcp::v6());
}

#[test]
fn test_tcp_resolve() {
    use IoService;
    use super::IpAddrV4;

    let io = IoService::new();
    let re = TcpResolver::new();
    for e in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(e.endpoint() == TcpEndpoint::new((IpAddrV4::new(127,0,0,1), 80)));
    }
}
