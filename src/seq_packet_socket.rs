use prelude::{Protocol, IoControl, GetSocketOption, SetSocketOption};
use ffi::{RawFd, AsRawFd, ioctl, getsockopt, setsockopt,
          getsockname, getpeername, socket, bind, shutdown};
use core::{IoContext, AsIoContext, Socket, AsyncFd};
use async::Handler;
use socket_base::{Shutdown, BytesReadable};
use reactive_io::{AsAsyncFd, getnonblock, setnonblock, cancel, connect, async_connect,
                  send, async_send, recv, async_recv};

use std::io;
use std::fmt;

/// Provides a sequenced packet socket.
pub struct SeqPacketSocket<P> {
    pro: P,
    fd: AsyncFd,
}

impl<P: Protocol> SeqPacketSocket<P> {
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<SeqPacketSocket<P>> {
        let fd = try!(socket(&pro));
        Ok(unsafe { Self::from_raw_fd(ctx, pro, fd) })
    }

    pub fn async_connect<F>(&self, ep: &P:: Endpoint, handler: F) -> F::Output
        where F: Handler<(), io::Error>,
    {
        async_connect(self, ep, handler)
    }

    pub fn async_receive<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize, io::Error>,
    {
        async_recv(self, buf, flags, handler)
    }

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize, io::Error>,
    {
        async_send(self, buf, flags, handler)
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = BytesReadable::default();
        try!(self.io_control(&mut bytes));
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    pub fn cancel(&self) -> &Self {
        cancel(self);
        self
    }

    pub fn connect(&self, ep: &P:: Endpoint) -> io::Result<()> {
        connect(self, ep)
    }

    pub fn get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    pub fn get_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>,
    {
        getsockopt(self, &self.pro)
    }

    pub fn io_control<T>(&self, cmd: &mut T) -> io::Result<()>
        where T: IoControl,
    {
        ioctl(self, cmd)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self, &self.pro)
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self, &self.pro)
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags)
    }

    pub fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }

    pub fn set_option<C>(&self, cmd: C) -> io::Result<()>
        where C: SetSocketOption<P>,
    {
        setsockopt(self, &self.pro, cmd)
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        shutdown(self, how)
    }
}

impl<P: Protocol> fmt::Debug for SeqPacketSocket<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SeqPacketSocket({:?})", self.pro)
    }
}

impl<P> AsRawFd for SeqPacketSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

unsafe impl<P> Send for SeqPacketSocket<P> {
}

unsafe impl<P> AsIoContext for SeqPacketSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        self.fd.as_ctx()
    }
}

impl<P: Protocol> Socket<P> for SeqPacketSocket<P> {
    unsafe fn from_raw_fd(ctx: &IoContext, pro: P, fd: RawFd) -> SeqPacketSocket<P> {
        SeqPacketSocket {
            pro: pro,
            fd: AsyncFd::new::<Self>(fd, ctx),
        }
    }

    fn protocol(&self) -> P {
        self.pro.clone()
    }
}

impl<P: Protocol> AsAsyncFd for SeqPacketSocket<P> {
    fn as_fd(&self) -> &AsyncFd {
        &self.fd
    }
}
