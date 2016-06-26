use std::io;
use std::mem;
use std::cell::Cell;
use {IoObject, IoService, Strand, Cancel};
use backbone::EpollIoActor;
use socket::*;
use socket::local::*;
use ops::*;
use ops::async::*;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalStream;

impl Protocol for LocalStream {
    fn family_type(&self) -> i32 {
        AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl Endpoint<LocalStream> for LocalEndpoint<LocalStream> {
    fn protocol(&self) -> LocalStream {
        LocalStream
    }
}

pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

pub struct LocalStreamSocket {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl LocalStreamSocket {
    pub fn new(io: &IoService) -> io::Result<Self> {
        let fd = try!(socket(LocalStream));
        Ok(LocalStreamSocket {
            actor: EpollIoActor::new(io, fd),
            nonblock: Cell::new(false),
        })
    }
}

impl IoObject for LocalStreamSocket {
    fn io_service(&self) -> &IoService {
        self.actor.io_service()
    }
}

impl AsRawFd for LocalStreamSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for LocalStreamSocket {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl NonBlocking for LocalStreamSocket {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl Socket for LocalStreamSocket {
    type Protocol = LocalStream;
    type Endpoint = LocalStreamEndpoint;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl ReadWrite for LocalStreamSocket {
    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        read_syncd(self, buf)
    }

    fn async_read_some<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: FnOnce(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj.get_mut());
        read_async(soc, buf, callback, obj)
    }

    fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        write_syncd(self, buf)
    }

    fn async_write_some<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, buf) = a(obj);
        write_async(soc, buf, callback, obj)
    }

    fn async_read_until<A, C, F, T>(a: A, cond: C, callback: F, obj: &Strand<T>)
        where A: FnOnce(&mut T) -> (&Self, &mut StreamBuf) + Send + 'static,
              C: MatchCondition + Clone + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, sbuf) = a(obj.get_mut());
        read_until_async(soc, sbuf, cond, callback, obj);
    }

    fn async_write_until<A, C, F, T>(a: A, cond: C, callback: F, obj: &Strand<T>)
        where A: FnOnce(&mut T) -> (&Self, &mut StreamBuf) + Send + 'static,
              C: MatchCondition + Clone + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        let (soc, sbuf) = a(obj.get_mut());
        write_until_async(soc, sbuf, cond, callback, obj);
    }
}

impl Cancel for LocalStreamSocket {
    fn cancel(&self) {
        cancel_io(self)
    }
}

impl SocketConnector for LocalStreamSocket {
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

impl SendRecv for LocalStreamSocket {
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

impl StreamSocket for LocalStreamSocket {
}

pub struct LocalStreamListener {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl LocalStreamListener {
    pub fn new(io: &IoService) -> io::Result<Self> {
        let fd = try!(socket(LocalStream));
        Ok(LocalStreamListener {
            actor: EpollIoActor::new(io, fd),
            nonblock: Cell::new(false),
        })
    }
}

impl IoObject for LocalStreamListener {
    fn io_service(&self) -> &IoService {
        self.actor.io_service()
    }
}

impl AsRawFd for LocalStreamListener {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for LocalStreamListener {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl NonBlocking for LocalStreamListener {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl Socket for LocalStreamListener {
    type Protocol = LocalStream;
    type Endpoint = LocalStreamEndpoint;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl SocketListener for LocalStreamListener {
    type Socket = LocalStreamSocket;

    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)> {
        let (fd, ep) = try!(accept_syncd(self, unsafe { mem::uninitialized() }));
        Ok((LocalStreamSocket {
            actor: EpollIoActor::new(self.io_service(), fd),
            nonblock: Cell::new(false),
        }, ep))
    }

    fn async_accept<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<(Self::Socket, Self::Endpoint)>) + Send + 'static,
              T: 'static {
        let soc = a(obj);
        accept_async(soc, unsafe { mem::uninitialized() },
                     move |obj, res| {
                         let io = obj.io_service().clone();
                         match res {
                             Ok((fd, ep)) =>
                                 callback(obj, Ok((LocalStreamSocket {
                                     actor: EpollIoActor::new(&io, fd),
                                     nonblock: Cell::new(false),
                                 }, ep))),
                             Err(err) => callback(obj, Err(err)),
                         }
                     }, obj);
    }
}

impl Cancel for LocalStreamListener {
    fn cancel(&self) {
        cancel_io(self)
    }
}

#[test]
fn test_stream() {
    assert!(LocalStream == LocalStream);
}
