use std::io;
use std::mem;
use {IoObject, IoService, Strand};
use backbone::EpollIoActor;
use socket::{Protocol, Endpoint, SocketBase, DgramSocket};
use ops::*;
use ops::async::*;
use super::LocalEndpoint;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalDgram;

impl Protocol for LocalDgram {
    fn family_type(&self) -> i32 {
        AF_UNIX
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
    io: IoService,
    actor: EpollIoActor,
}

impl Drop for LocalDgramSocket {
    fn drop(&mut self) {
        let _ = self.actor.unset_in(&self.io);
        let _ = self.actor.unset_out(&self.io);
        let _ = close(self);
    }
}

impl IoObject for LocalDgramSocket {
    fn io_service(&self) -> IoService {
        self.io.clone()
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

impl SocketBase<LocalDgram> for LocalDgramSocket {
    type Endpoint = LocalDgramEndpoint;

    fn new(io: &IoService, pro: LocalDgram) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(LocalDgramSocket {
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

impl DgramSocket<LocalDgram> for LocalDgramSocket {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()> {
        connect(self, ep)
    }

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static
    {
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
              T: 'static
    {
        async_recv(a, flags, callback, obj)
    }

    fn recv_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, Self::Endpoint)> {
        recvfrom(self, buf, flags, unsafe { mem::uninitialized() })
    }

    fn async_recv_from<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<(usize, Self::Endpoint)>) + Send + 'static,
              T: 'static
    {
        async_recvfrom(a, flags, unsafe { mem::uninitialized() }, callback, obj)
    }

    fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags)
    }

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static
    {
        async_send(a, flags, callback, obj)
    }

    fn send_to(&self, buf: &[u8], flags: i32, ep: &Self::Endpoint) -> io::Result<usize> {
        sendto(self, buf, flags, ep)
    }

    fn async_send_to<A, F, T>(a: A, flags: i32, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static
    {
        async_sendto(a, flags, ep, callback, obj)
    }

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self
    {
        cancel_io(a, obj)
    }
}
