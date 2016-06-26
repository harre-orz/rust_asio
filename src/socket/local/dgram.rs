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
pub struct LocalDgram;

impl Protocol for LocalDgram {
    fn family_type(&self) -> i32 {
        AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl Endpoint<LocalDgram> for LocalEndpoint<LocalDgram> {
    fn protocol(&self) -> LocalDgram {
        LocalDgram
    }
}

pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

pub struct LocalDgramSocket {
    actor: EpollIoActor,
    nonblock: Cell<bool>,
}

impl LocalDgramSocket {
    pub fn new(io: &IoService) -> io::Result<Self> {
        let fd = try!(socket(LocalDgram));
        Ok(LocalDgramSocket {
            actor: EpollIoActor::new(io, fd),
            nonblock: Cell::new(false),
        })
    }
}

impl IoObject for LocalDgramSocket {
    fn io_service(&self) -> &IoService {
        self.actor.io_service()
    }
}

impl AsRawFd for LocalDgramSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for LocalDgramSocket {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl NonBlocking for LocalDgramSocket {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl Socket for LocalDgramSocket {
    type Protocol = LocalDgram;
    type Endpoint = LocalDgramEndpoint;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl SendRecv for LocalDgramSocket {
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

impl SendToRecvFrom for LocalDgramSocket {
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

impl Cancel for LocalDgramSocket {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self {
        cancel_io(a(obj), obj)
    }
}

impl SocketConnector for LocalDgramSocket {
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

impl DgramSocket for LocalDgramSocket {
}

#[test]
fn test_dgram() {
    assert!(LocalDgram == LocalDgram);
}
