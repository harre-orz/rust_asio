use std::io;
use std::mem;
use std::cell::Cell;
use {Strand, Cancel};
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
    pub fn new() -> io::Result<Self> {
        let fd = try!(socket(LocalStream));
        Ok(LocalStreamSocket {
            actor: EpollIoActor::new(fd),
            nonblock: Cell::new(false),
        })
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

impl Cancel for LocalStreamSocket {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + 'static,
              T: 'static {
        cancel_io(a, obj)
    }
}

impl SocketConnector for LocalStreamSocket {
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

impl SendRecv for LocalStreamSocket {
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

impl StreamSocket for LocalStreamSocket {
}

pub struct LocalStreamListener {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl LocalStreamListener {
    pub fn new() -> io::Result<Self> {
        let fd = try!(socket(LocalStream));
        Ok(LocalStreamListener {
            actor: EpollIoActor::new(fd),
            nonblock: Cell::new(false),
        })
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
                                 callback(obj, Ok((LocalStreamSocket {
                                     actor: EpollIoActor::new(fd),
                                     nonblock: Cell::new(false),
                                 }, ep))),
                             Err(err) => callback(obj, Err(err)),
                         }
                     }, obj);
    }
}

impl Cancel for LocalStreamListener {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + 'static,
              T: 'static {
        cancel_io(a, obj)
    }
}

#[test]
fn test_stream() {
    assert!(LocalStream == LocalStream);
}
