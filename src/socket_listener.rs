use prelude::{Protocol, IoControl, GetSocketOption, SetSocketOption};
use ffi::{RawFd, AsRawFd, socket, bind, listen, ioctl, getsockopt, setsockopt,
          getsockname};
use core::{IoContext, AsIoContext, Socket, AsyncFd};
use async::Handler;
use reactive_io::{AsAsyncFd, getnonblock, setnonblock, accept, async_accept, cancel};
use socket_base::MAX_CONNECTIONS;

use std::io;
use std::fmt;
use std::marker::PhantomData;

/// Provides an ability to accept new connections.
pub struct SocketListener<P, S> {
    pro: P,
    fd: AsyncFd,
    _marker: PhantomData<S>,
}

impl<P: Protocol, S: Socket<P>> SocketListener<P, S> {
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<SocketListener<P, S>> {
        let fd = try!(socket(&pro));
        Ok(unsafe { Self::from_raw_fd(ctx, pro, fd) })
    }

    pub fn accept(&self) -> io::Result<(S, P::Endpoint)> {
        accept(self, &self.pro)
    }

    pub fn async_accept<F>(&self, handler: F) -> F::Output
        where F: Handler<(S, P::Endpoint), io::Error>,
    {
        async_accept(self, self.protocol(), handler)
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    pub fn cancel(&self) -> &Self {
        cancel(self);
        self
    }

    pub fn listen(&self) -> io::Result<()> {
        listen(self, MAX_CONNECTIONS)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self, &self.pro)
    }

    pub fn io_control<T>(&self, cmd: &mut T) -> io::Result<()>
        where T: IoControl,
    {
        ioctl(self, cmd)
    }

    pub fn get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    pub fn get_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>,
    {
        getsockopt(self, &self.pro)
    }

    pub fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }

    pub fn set_option<C>(&self, cmd: C) -> io::Result<()>
        where C: SetSocketOption<P>,
    {
        setsockopt(self, &self.pro, cmd)
    }
}

impl<P: Protocol, S> fmt::Debug for SocketListener<P, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SocketListener({:?})", self.pro)
    }
}

impl<P, S> AsRawFd for SocketListener<P, S> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

unsafe impl<P, S> Send for SocketListener<P, S> { }

unsafe impl<P, S> AsIoContext for SocketListener<P, S> {
    fn as_ctx(&self) -> &IoContext {
        self.fd.as_ctx()
    }
}

impl<P: Protocol, S: Socket<P>> Socket<P> for SocketListener<P, S> {
    unsafe fn from_raw_fd(ctx: &IoContext, pro: P, fd: RawFd) -> SocketListener<P, S> {
        SocketListener {
            pro: pro,
            fd: AsyncFd::new::<Self>(fd, ctx),
            _marker: PhantomData,
        }
    }

    fn protocol(&self) -> P {
        self.pro.clone()
    }
}

impl<P: Protocol, S: Socket<P>> AsAsyncFd for SocketListener<P, S> {
    fn as_fd(&self) -> &AsyncFd {
        &self.fd
    }
}
