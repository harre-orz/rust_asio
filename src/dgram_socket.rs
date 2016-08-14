use std::io;
use std::mem;
use {IoObject, IoService, Protocol, IoControl, GetSocketOption, SetSocketOption, Shutdown, FromRawFd, Handler};
use socket_base::{AtMark, BytesReadable};
use backbone::{RawFd, AsRawFd, IoActor, AsIoActor, socket, bind, shutdown,
               ioctl, getsockopt, setsockopt, getsockname, getpeername, getnonblock, setnonblock};
use backbone::ops::{connect, recv, recvfrom, send, sendto,
                    async_connect, async_recv, async_recvfrom, async_send, async_sendto, cancel_io};

pub struct DgramSocket<P> {
    pro: P,
    io: IoActor,
}

impl<P: Protocol> DgramSocket<P> {
    pub fn new<T: IoObject>(io: &T, pro: P) -> io::Result<DgramSocket<P>> {
        let fd = try!(socket(&pro));
        Ok(unsafe { Self::from_raw_fd(io, pro, fd) })
    }

    pub fn at_mark(&self) -> io::Result<bool> {
        let mut mark = AtMark::default();
        try!(self.io_control(&mut mark));
        Ok(mark.get())
    }

    pub fn async_connect<F: Handler<Self, ()>>(&self, ep: &P:: Endpoint, handler: F) {
        async_connect(self, ep, handler)
    }

    pub fn async_receive<F: Handler<Self, usize>>(&self, buf: &mut [u8], flags: i32, handler: F) {
        async_recv(self, buf, flags, handler)
    }

    pub fn async_receive_from<F: Handler<Self, (usize, P::Endpoint)>>(&self, buf: &mut [u8], flags: i32, handler: F) {
        async_recvfrom(self, buf, flags, unsafe { mem::uninitialized() }, handler)
    }

    pub fn async_send<F: Handler<Self, usize>>(&self, buf: &[u8], flags: i32, handler: F) {
        async_send(self, buf, flags, handler)
    }

    pub fn async_send_to<F: Handler<Self, usize>>(&self, buf: &[u8], flags: i32, ep: P::Endpoint, handler: F) {
        async_sendto(self, buf, flags, ep, handler)
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = BytesReadable::default();
        try!(self.io_control(&mut bytes));
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    pub fn cancel(&self) {
        cancel_io(self)
    }

    pub fn connect(&self, ep: &P:: Endpoint) -> io::Result<()> {
        connect(self, ep)
    }

    pub fn get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    pub fn get_option<C: GetSocketOption<P>>(&self) -> io::Result<C> {
        getsockopt(self, &self.pro)
    }

    pub fn io_control<C: IoControl>(&self, cmd: &mut C) -> io::Result<()> {
        ioctl(self, cmd)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }

    pub fn protocol(&self) -> P {
        self.pro.clone()
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags)
    }

    pub fn receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        recvfrom(self, buf, flags, unsafe { mem::uninitialized() })
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self, unsafe { mem::uninitialized() })
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags)
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: P::Endpoint) -> io::Result<usize> {
        sendto(self, buf, flags, ep)
    }

    pub fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }

    pub fn set_option<C: SetSocketOption<P>>(&self, cmd: C) -> io::Result<()> {
        setsockopt(self, &self.pro, cmd)
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        shutdown(self, how)
    }
}

impl<P> IoObject for DgramSocket<P> {
    fn io_service(&self) -> &IoService {
        self.io.io_service()
    }
}

impl<P: Protocol> FromRawFd<P> for DgramSocket<P> {
    unsafe fn from_raw_fd<T: IoObject>(io: &T, pro: P, fd: RawFd) -> DgramSocket<P> {
        DgramSocket {
            pro: pro,
            io: IoActor::new(io, fd),
        }
    }
}

impl<P> AsRawFd for DgramSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.io.as_raw_fd()
    }
}

impl<P: Protocol> AsIoActor for DgramSocket<P> {
    fn as_io_actor(&self) -> &IoActor {
        &self.io
    }
}
