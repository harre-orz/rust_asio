use std::io;
use std::mem;
use {IoObject, IoService, Strand};
use backbone::EpollIoActor;
use socket::{Protocol, Endpoint, SocketBase, SeqPacketSocket, SocketListener};
use ops::*;
use ops::async::*;
use super::LocalEndpoint;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalSeqPacket;

impl Protocol for LocalSeqPacket {
    fn family_type(&self) -> i32 {
        AF_UNIX
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
    io: IoService,
    actor: EpollIoActor,
}

impl Drop for LocalSeqPacketSocket {
    fn drop(&mut self) {
        let _ = self.actor.unset_in(&self.io);
        let _ = self.actor.unset_out(&self.io);
        let _ = close(self);
    }
}

impl IoObject for LocalSeqPacketSocket {
    fn io_service(&self) -> IoService {
        self.io.clone()
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

impl SocketBase<LocalSeqPacket> for LocalSeqPacketSocket {
    type Endpoint = LocalSeqPacketEndpoint;

    fn new(io: &IoService, pro: LocalSeqPacket) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(LocalSeqPacketSocket {
            io: io.clone(),
            actor: EpollIoActor::new(fd)
        })
    }

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl SeqPacketSocket<LocalSeqPacket> for LocalSeqPacketSocket {
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

pub struct LocalSeqPacketListener {
    io: IoService,
    actor: EpollIoActor,
}

impl Drop for LocalSeqPacketListener {
    fn drop(&mut self) {
        let _ = self.actor.unset_in(&self.io);
        let _ = self.actor.unset_out(&self.io);
        let _ = close(self);
    }
}

impl IoObject for LocalSeqPacketListener {
    fn io_service(&self) -> IoService {
        self.io.clone()
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

impl SocketBase<LocalSeqPacket> for LocalSeqPacketListener {
    type Endpoint = LocalSeqPacketEndpoint;

    fn new(io: &IoService, pro: LocalSeqPacket) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(LocalSeqPacketListener {
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

impl SocketListener<LocalSeqPacket> for LocalSeqPacketListener {
    type Socket = LocalSeqPacketSocket;

    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)> {
        let (io, fd, ep) = try!(accept(self, unsafe { mem::uninitialized() }));
        Ok((LocalSeqPacketSocket {
            io: io,
            actor: EpollIoActor::new(fd),
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
                                 callback(obj, Ok((LocalSeqPacketSocket {
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

// #[test]
// fn test_stream() {
//     assert!(Stream == Stream);
// }

// #[test]
// fn test_dgram() {
//     assert!(Dgram == Dgram);
// }

// #[test]
// fn test_seqpacket() {
//     assert!(SeqPacket == SeqPacket);
// }

// #[test]
// fn test_endpoint() {
//     let ep: Endpoint<Stream> = Endpoint::new("hello").unwrap();
//     assert!(ep.path() == "hello");
// }
