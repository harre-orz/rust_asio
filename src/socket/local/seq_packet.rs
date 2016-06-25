use std::io;
use std::mem;
use std::cell::Cell;
use {IoObject, Strand, Cancel};
use backbone::EpollIoActor;
use socket::*;
use socket::local::*;
use ops::*;
use ops::async::*;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalSeqPacket;

impl Protocol for LocalSeqPacket {
    fn family_type(&self) -> i32 {
        AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        SOCK_SEQPACKET
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl Endpoint<LocalSeqPacket> for LocalEndpoint<LocalSeqPacket> {
    fn protocol(&self) -> LocalSeqPacket {
        LocalSeqPacket
    }
}

pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

pub struct LocalSeqPacketSocket {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl LocalSeqPacketSocket {
    pub fn new() -> io::Result<Self> {
        let fd = try!(socket(LocalSeqPacket));
        Ok(LocalSeqPacketSocket {
            actor: EpollIoActor::new(fd),
            nonblock: Cell::new(false),
        })
    }
}

impl AsRawFd for LocalSeqPacketSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for LocalSeqPacketSocket {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl NonBlocking for LocalSeqPacketSocket {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl Socket for LocalSeqPacketSocket {
    type Protocol = LocalSeqPacket;
    type Endpoint = LocalSeqPacketEndpoint;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl SocketConnector for LocalSeqPacketSocket {
    fn connect<T: IoObject>(&self, io: &T, ep: &Self::Endpoint) -> io::Result<()> {
        connect_syncd(self, ep, io.io_service())
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

impl SendRecv for LocalSeqPacketSocket {
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

impl Cancel for LocalSeqPacketSocket {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self {
        cancel_io(a(obj), obj)
    }
}

pub struct LocalSeqPacketListener {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl LocalSeqPacketListener {
    pub fn new() -> io::Result<Self> {
        let fd = try!(socket(LocalSeqPacket));
        Ok(LocalSeqPacketListener {
            actor: EpollIoActor::new(fd),
            nonblock: Cell::new(false),
        })
    }
}

impl AsRawFd for LocalSeqPacketListener {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for LocalSeqPacketListener {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl NonBlocking for LocalSeqPacketListener {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}
impl Socket for LocalSeqPacketListener {
    type Protocol = LocalSeqPacket;
    type Endpoint = LocalSeqPacketEndpoint;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl SocketListener for LocalSeqPacketListener {
    type Socket = LocalSeqPacketSocket;

    fn accept<T: IoObject>(&self, io: &T) -> io::Result<(Self::Socket, Self::Endpoint)> {
        let (fd, ep) = try!(accept_syncd(self, unsafe { mem::uninitialized() }, io.io_service()));
        Ok((LocalSeqPacketSocket {
            actor: EpollIoActor::new(fd),
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
                         match res {
                             Ok((fd, ep)) =>
                                 callback(obj, Ok((LocalSeqPacketSocket {
                                     actor: EpollIoActor::new(fd),
                                     nonblock: Cell::new(false),
                                 }, ep))),
                             Err(err) => callback(obj, Err(err)),
                         }
                     }, obj);
    }
}

impl Cancel for LocalSeqPacketListener {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
    where A: FnOnce(&T) -> &Self {
        cancel_io(a(obj), obj)
    }
}

#[test]
fn test_seq_packet() {
    assert!(LocalSeqPacket == LocalSeqPacket);
}
