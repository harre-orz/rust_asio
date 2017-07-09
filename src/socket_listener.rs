use ffi::*;
use core::*;
use prelude::*;
use socket_base::MAX_CONNECTIONS;

use std::io;
use std::marker::PhantomData;
use std::time::Duration;

pub struct SocketListener<P, T, R> {
    soc: SocketContext<P>,
    _marker: PhantomData<(P, T, R)>,
}

impl<P, T, R> SocketListener<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<SocketListener<P, T, R>> {
        let fd = socket(&pro).map_err(error)?;
        Ok(SocketListener {
            soc: SocketContext::new(ctx, pro, fd),
            _marker: PhantomData,
        })
    }

    pub fn accept(&mut self) -> io::Result<(T, R, P::Endpoint)> {
        if self.soc.block {
            readable(self, &self.soc.recv_timeout).map_err(error)?;
        }
        let (fd, ep) = accept(self).map_err(error)?;
        let pro = self.protocol().clone();
        let (tx, rx) = PairBox::new(SocketContext::new(self.as_ctx(), pro, fd));
        Ok((T::from_ctx(tx), R::from_ctx(rx), ep))
    }

    pub fn bind(&mut self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep).map_err(error)
    }

    pub fn listen(&mut self) -> io::Result<()> {
        listen(self, MAX_CONNECTIONS).map_err(error)
    }

    pub fn get_timeout(&self) -> Option<Duration> {
        self.soc.recv_timeout.clone()
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.soc.recv_timeout = timeout;
    }
}

unsafe impl<P, T, R> AsIoContext for SocketListener<P, T, R> {
    fn as_ctx(&self) -> &IoContext {
        &self.soc.ctx
    }
}

impl<P, T, R> AsRawFd for SocketListener<P, T, R> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.fd
    }
}

impl<P, T, R> Socket<P> for SocketListener<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    fn protocol(&self) -> &P {
        &self.soc.pro
    }
}

impl<P, T, R> SocketControl<P> for SocketListener<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    fn get_non_blocking(&self) -> io::Result<bool> {
        self.soc.getnonblock()
    }

    fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
    {
        getsockopt(self).map_err(error)
    }

    fn io_control<C>(self, cmd: &mut C) -> io::Result<Self>
        where C: IoControl,
    {
        ioctl(&self, cmd).map_err(error)?;
        Ok(self)
    }

    fn set_non_blocking(self, on: bool) -> io::Result<Self> {
        self.soc.setnonblock(on)?;
        Ok(self)
    }

    fn set_socket_option<C>(self, cmd: C) -> io::Result<Self>
        where C: SetSocketOption<P>
    {
        setsockopt(&self, cmd).map_err(error)?;
        Ok(self)
    }
}
