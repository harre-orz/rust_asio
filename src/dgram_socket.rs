#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, InnerSocket, IoContext, Perform, ThreadIoContext};
use handler::{ Handler, Yield};
use ops::{AsyncConnect, AsyncRecv, AsyncRecvFrom, AsyncSend, AsyncSendTo, AsyncSocketOp};
use socket_base;

use std::io;
use std::fmt;

pub struct DgramSocket<P> {
    inner: Box<InnerSocket<P>>,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro)?;
        Ok(unsafe { Self::from_raw_fd(ctx, soc, pro) })
    }

    pub fn async_connect<F>(&self, ep: &P::Endpoint, handler: F) -> F::Output
    where
        F: Handler<(), io::Error>,
    {
        let (tx, rx) = handler.channel();
        self.as_ctx()
            .do_post(AsyncConnect::new(self, ep.clone(), tx));
        rx.yield_return()
    }

    pub fn async_receive<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        self.as_ctx()
            .do_dispatch(AsyncRecv::new(self, buf, flags, tx));
        rx.yield_return()
    }

    pub fn async_receive_from<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<(usize, P::Endpoint), io::Error>,
    {
        let (tx, rx) = handler.channel();
        self.as_ctx()
            .do_dispatch(AsyncRecvFrom::new(self, buf, flags, tx));
        rx.yield_return()
    }

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        self.as_ctx()
            .do_dispatch(AsyncSend::new(self, buf, flags, tx));
        rx.yield_return()
    }

    pub fn async_send_to<F>(
        &self,
        buf: &[u8],
        flags: i32,
        ep: &P::Endpoint,
        handler: F,
    ) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        self.as_ctx()
            .do_dispatch(AsyncSendTo::new(self, buf, flags, ep.clone(), tx));
        rx.yield_return()
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = socket_base::BytesReadable::default();
        ioctl(self, &mut bytes)?;
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn cancel(&mut self) {
        self.inner.cancel()
    }

    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        if self.as_ctx().stopped() {
            return Err(OPERATION_CANCELED.into());
        }
        match connect(self, ep) {
            Ok(_) => Ok(()),
            Err(IN_PROGRESS) | Err(WOULD_BLOCK) => Ok(writable(self, &Timeout::default())?),
            Err(err) => Err(err.into()),
        }
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self)?)
    }

    pub fn get_option<C>(&self) -> io::Result<C>
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

    pub fn nonblocking_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            Ok(recv(self, buf, flags)?)
        }
    }

    pub fn nonblocking_receive_from(
        &self,
        buf: &mut [u8],
        flags: i32,
    ) -> io::Result<(usize, P::Endpoint)> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            unsafe {
                let mut ep = self.protocol().uninitialized();
                ep.resize(0);
                return Ok((0, ep));
            }
        } else {
            Ok(recvfrom(self, buf, flags)?)
        }
    }

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            Ok(send(self, buf, flags)?)
        }
    }

    pub fn nonblocking_send_to(
        &self,
        buf: &[u8],
        flags: i32,
        ep: &P::Endpoint,
    ) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            Ok(sendto(self, buf, flags, ep)?)
        }
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        while !self.as_ctx().stopped() {
            match recv(self, buf, flags) {
                Ok(len) => return Ok(len),
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

    pub fn receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        if buf.is_empty() {
            unsafe {
                let mut ep = self.protocol().uninitialized();
                ep.resize(0);
                return Ok((0, ep));
            }
        }

        while !self.as_ctx().stopped() {
            match recvfrom(self, buf, flags) {
                Ok(len) => return Ok(len),
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

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getpeername(self)?)
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        while !self.as_ctx().stopped() {
            match send(self, buf, flags) {
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) => (),
                Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                    if let Err(err) = writable(self, &Timeout::default()) {
                        return Err(err.into());
                    }
                }
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        while !self.as_ctx().stopped() {
            match sendto(self, buf, flags, ep) {
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) => (),
                Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                    if let Err(err) = writable(self, &Timeout::default()) {
                        return Err(err.into());
                    }
                }
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn set_option<C>(&self, cmd: C) -> io::Result<()>
    where
        C: SetSocketOption<P>,
    {
        Ok(setsockopt(self, cmd)?)
    }

    pub fn shutdown(&self, how: socket_base::Shutdown) -> io::Result<()> {
        Ok(shutdown(self, how)?)
    }
}

unsafe impl<P> Send for DgramSocket<P> {}

unsafe impl<P> AsIoContext for DgramSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }
}

impl<P> AsRawFd for DgramSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl<P> Socket<P> for DgramSocket<P>
where
    P: Protocol,
{
    fn protocol(&self) -> &P {
        self.inner.protocol()
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        DgramSocket {
            inner: InnerSocket::new(ctx, soc, pro),
        }
    }
}

impl<P> fmt::Debug for DgramSocket<P>
where
    P: Protocol + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})", self.protocol(), self.as_raw_fd())
    }
}

impl<P> AsyncSocketOp for DgramSocket<P>
where
    P: Protocol,
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
