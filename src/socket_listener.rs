use prelude::*;
use ffi::*;
use core::{IoContext, AsIoContext, ThreadIoContext, SocketImpl};
use socket_base;

use std::io;
use std::marker::PhantomData;


pub struct SocketListener<P, S, M> {
    soc: Box<SocketImpl<P>>,
    _marker: PhantomData<(S, M)>,
}

impl<P, S, M> SocketListener<P, S, M>
    where P: Protocol,
          S: Socket<P>,
          M: Send + 'static,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro)?;
        Ok(SocketListener {
            soc: SocketImpl::new(ctx, pro, soc),
            _marker: PhantomData,
        })
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep).map_err(From::from)
    }

    pub fn listen(&self) -> io::Result<()> {
        listen(self, socket_base::MAX_CONNECTIONS).map_err(From::from)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self).map_err(From::from)
     }

    pub fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
    {
        getsockopt(self).map_err(From::from)
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
        where C: IoControl
    {
        ioctl(self, cmd).map_err(From::from)
    }

    pub fn set_socket_option<C>(&self, cmd: C) -> io::Result<()>
        where C: SetSocketOption<P>
    {
        setsockopt(self, cmd).map_err(From::from)
    }
}

impl<P, S> SocketListener<P, S, socket_base::Sync>
    where P: Protocol,
          S: Socket<P>,
{
    pub fn accept(&self) -> io::Result<(S, P::Endpoint)> {
        while !self.as_ctx().stopped() {
            match accept(self) {
                Ok((soc, ep)) => {
                    let pro = self.protocol().clone();
                    let soc = unsafe { S::from_raw_fd(self.as_ctx(), pro, soc) };
                    return Ok((soc, ep))
                },
                Err(INTERRUPTED) | Err(WOULD_BLOCK) =>
                    if let Err(err) = readable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn nonblicking_accept(&self) -> io::Result<(S, P::Endpoint)> {
        if self.as_ctx().stopped() {
            return Err(OPERATION_CANCELED.into())
        } else {
            match accept(self) {
                Ok((soc, ep)) => {
                    let pro = self.protocol().clone();
                    let soc = unsafe { S::from_raw_fd(self.as_ctx(), pro, soc) };
                    Ok((soc, ep))
                },
                Err(err) => Err(err.into()),
            }
        }
    }
}

unsafe impl<P, S, M> AsIoContext for SocketListener<P, S, M> {
    fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }
}

impl<P, S, M> AsRawFd for SocketListener<P, S, M> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.as_raw_fd()
    }
}

impl<P, S, M> Socket<P> for SocketListener<P, S, M>
    where P: Protocol,
          S: Socket<P>,
          M: Send + 'static,
{
    fn protocol(&self) -> &P {
        self.soc.protocol()
    }

    unsafe fn from_raw_fd(ctx: &IoContext, pro: P, soc: RawFd) -> Self {
        SocketListener {
            soc: SocketImpl::new(ctx, pro, soc),
            _marker: PhantomData,
        }
    }
}
