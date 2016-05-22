use std::io;
use std::mem;
use {IoObject, IoService, Strand};
use backbone::EpollIoActor;
use socket::{Protocol, Endpoint, ReadWrite, ResolveQuery, Resolver, SocketBase, StreamSocket, SocketListener};
use ops::*;
use ops::async::*;
use super::IpEndpoint;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Tcp {
    family: i32,
}

impl Tcp {
    pub fn v4() -> Tcp {
        Tcp { family: AF_INET as i32 }
    }

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

pub type TcpEndpoint = IpEndpoint<Tcp>;

pub struct TcpSocket {
    io: IoService,
    actor: EpollIoActor,
}

impl Drop for TcpSocket {
    fn drop(&mut self) {
        let _ = self.actor.unset_in(&self.io);
        let _ = self.actor.unset_out(&self.io);
        let _ = close(self);
    }
}

impl IoObject for TcpSocket {
    fn io_service(&self) -> IoService {
        self.io.clone()
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

impl SocketBase<Tcp> for TcpSocket {
    type Endpoint = TcpEndpoint;

    fn new(io: &IoService, pro: Tcp) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(TcpSocket {
            io: io.clone(),
            actor: EpollIoActor::new(fd),
        })
    }

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl ReadWrite for TcpSocket {
    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        recv(self, buf, 0)
    }

    fn async_read_some<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        async_recv(a, flags, callback, obj)
    }

    fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        send(self, buf, 0)
    }

    fn async_write_some<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        async_send(a, flags, callback, obj)
    }

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self {
        cancel_io(a, obj)
    }
}

impl StreamSocket<Tcp> for TcpSocket {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()> {
        connect(self, ep)
    }

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        async_connect(a, ep, callback, obj)
    }

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint> {
        getpeername(self, unsafe { mem::uninitialized() })
    }

    fn recv(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags)
    }

    fn async_recv<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        async_recv(a, flags, callback, obj)
    }

    fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags)
    }

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        async_send(a, flags, callback, obj)
    }

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self {
        cancel_io(a, obj)
    }
}

pub struct TcpListener {
    io: IoService,
    actor: EpollIoActor,
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        let _ = self.actor.unset_in(&self.io);
        let _ = self.actor.unset_out(&self.io);
        let _ = close(self);
    }
}

impl IoObject for TcpListener {
    fn io_service(&self) -> IoService {
        self.io.clone()
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

impl SocketBase<Tcp> for TcpListener {
    type Endpoint = TcpEndpoint;

    fn new(io: &IoService, pro: Tcp) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(TcpListener {
            io: io.clone(),
            actor: EpollIoActor::new(fd),
        })
    }

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl SocketListener<Tcp> for TcpListener {
    type Socket = TcpSocket;

    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)> {
        let (io, fd, ep) = try!(accept(self, unsafe { mem::uninitialized() }));
        Ok((TcpSocket {
            io: io,
            actor: EpollIoActor::new(fd)
        }, ep))
    }

    fn async_accept<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<(Self::Socket, Self::Endpoint)>) + Send + 'static,
              T: 'static {
        async_accept(a, unsafe { mem::uninitialized() },
                     move |obj, res| {
                         match res {
                             Ok((io, fd, ep)) =>
                                 callback(obj, Ok((TcpSocket {
                                     io: io,
                                     actor: EpollIoActor::new(fd),
                                 }, ep))),
                             Err(err) => callback(obj, Err(err)),
                         }
                     }, obj);
    }

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self {
        cancel_io(a, obj)
    }
}

pub struct TcpResolver {
    io: IoService,
}

impl IoObject for TcpResolver {
    fn io_service(&self) -> IoService {
        self.io.clone()
    }
}

impl Resolver<Tcp> for TcpResolver {
    fn new(io: &IoService) -> Self {
        TcpResolver {
            io: io.clone(),
        }
    }

    fn resolve<'a, Q: ResolveQuery<'a, Tcp>>(&self, query: Q) -> io::Result<Q::Iter> {
        query.query(Tcp { family: AF_UNSPEC })
    }

    fn async_resolve<'a, Q, A, F, T>(a: A, query: Q, callback: F, obj: &Strand<T>)
        where Q: ResolveQuery<'a, Tcp> + 'static,
              A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<Q::Iter>) + Send + 'static,
              T: 'static {
        let io = a(&*obj).io_service();
        let _obj = obj.clone();
        io.post_strand(move || {
            let res = a(&*_obj);
            callback(&_obj, res.resolve(query));
        }, obj)
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
    use super::IpAddrV4;
    let io = IoService::new();
    let re = TcpResolver::new(&io);
    for e in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(e.endpoint() == TcpEndpoint::new((IpAddrV4::new(127,0,0,1), 80)));
    }
}
