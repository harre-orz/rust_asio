#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{IoContext, AsIoContext, ThreadIoContext, Perform, InnerSocket};
use async::{Handler, AsyncAccept, Yield, AsyncSocketOp};
use socket_base;

use std::io;
use std::marker::PhantomData;


pub struct SocketListener<P, S> {
    inner: Box<InnerSocket<P>>,
    _marker: PhantomData<S>,
}

impl<P, S> SocketListener<P, S>
where
    P: Protocol,
    S: Socket<P>,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro)?;
        Ok(unsafe { Self::from_raw_fd(ctx, soc, pro) })
    }

    pub fn accept(&self) -> io::Result<(S, P::Endpoint)> {
        while !self.as_ctx().stopped() {
            match accept(self) {
                Ok((acc, ep)) => {
                    let pro = self.protocol().clone();
                    let acc = unsafe { S::from_raw_fd(self.as_ctx(), acc, pro) };
                    return Ok((acc, ep));
                }
                Err(INTERRUPTED) => (),
                Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                    if let Err(err) = readable(self, &Timeout::default()) {
                        return Err(err.into());
                    }
                }
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn async_accept<F>(&self, handler: F) -> F::Output
    where
        F: Handler<(S, P::Endpoint), io::Error>,
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_post(AsyncAccept::new(self, tx));
        rx.yield_return()
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn cancel(&mut self) {
        self.inner.cancel()
    }

    pub fn listen(&self) -> io::Result<()> {
        Ok(listen(self, socket_base::MAX_CONNECTIONS)?)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self)?)
    }

    pub fn nonblicking_accept(&self) -> io::Result<(S, P::Endpoint)> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else {
            Ok(accept(self).map(|(acc, ep)| {
                let pro = self.protocol().clone();
                let acc = unsafe { S::from_raw_fd(self.as_ctx(), acc, pro) };
                (acc, ep)
            })?)
        }
    }

    pub fn get_socket_option<C>(&self) -> io::Result<C>
    where
        C: GetSocketOption<P>,
    {
        Ok(getsockopt(self)?)
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
    where
        C: IoControl,
    {
        Ok(ioctl(self, cmd)?)
    }

    pub fn set_socket_option<C>(&self, cmd: C) -> io::Result<()>
    where
        C: SetSocketOption<P>,
    {
        Ok(setsockopt(self, cmd)?)
    }
}

unsafe impl<P, S> Send for SocketListener<P, S> {}

unsafe impl<P, S> AsIoContext for SocketListener<P, S> {
    fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }
}

impl<P, S> AsRawFd for SocketListener<P, S> {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl<P, S> Socket<P> for SocketListener<P, S>
where
    P: Protocol,
    S: Socket<P>,
{
    fn protocol(&self) -> &P {
        self.inner.protocol()
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        SocketListener {
            inner: InnerSocket::new(ctx, soc, pro),
            _marker: PhantomData,
        }
    }
}

impl<P, S> AsyncSocketOp for SocketListener<P, S>
where
    P: Protocol,
    S: Socket<P>,
{
    fn add_read_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.inner.add_read_op(this, op, err)
    }

    fn add_write_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.inner.add_write_op(this, op, err)
    }

    fn next_read_op(&mut self, this: &mut ThreadIoContext) {
        self.inner.next_read_op(this)
    }

    fn next_write_op(&mut self, this: &mut ThreadIoContext) {
        self.inner.next_write_op(this)
    }
}
