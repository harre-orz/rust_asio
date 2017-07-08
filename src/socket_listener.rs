use ffi::*;
use core::*;
use prelude::*;
use socket_base::MAX_CONNECTIONS;

use std::io;
use std::marker::PhantomData;

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
            soc: SocketContext {
                ctx: ctx.clone(),
                pro: pro,
                fd: fd,
            },
            _marker: PhantomData,
        })
    }

    pub fn accept(&self) -> io::Result<(T, R, P::Endpoint)> {
        let (fd, ep) = accept(self).map_err(error)?;
        let (tx, rx) = PairBox::new(SocketContext {
            ctx: self.as_ctx().clone(),
            pro: self.protocol().clone(),
            fd: fd,
        });
        Ok((T::from_ctx(tx), R::from_ctx(rx), ep))
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep).map_err(error)
    }

    pub fn listen(&self) -> io::Result<()> {
        listen(self, MAX_CONNECTIONS).map_err(error)
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
