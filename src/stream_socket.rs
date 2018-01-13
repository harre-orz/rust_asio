#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{IoContext, AsIoContext, ThreadIoContext, Perform, SocketImpl, AsyncSocket};
use async::{Handler, AsyncConnect, AsyncRead, AsyncRecv, AsyncSend, AsyncWrite, Yield};
use streams::Stream;
use socket_base;

use std::io;


pub struct StreamSocket<P> {
    soc: SocketImpl<P>,
}

impl<P> StreamSocket<P>
    where P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro)?;
        Ok(unsafe { Self::from_raw_fd(ctx, soc, pro) })
    }

    pub fn async_connect<F>(&self, ep: &P::Endpoint, handler: F) -> F::Output
        where F: Handler<(), io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_post(AsyncConnect::new(self, ep.clone(), tx));
        rx.yield_return()
    }

    pub fn async_recv<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncRecv::new(self, buf, flags, tx));
        rx.yield_return()
    }

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncSend::new(self, buf, flags, tx));
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
        self.soc.cancel();
    }

    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        if self.as_ctx().stopped() {
            return Err(OPERATION_CANCELED.into())
        }
        match connect(self, ep) {
            Ok(_) =>
                Ok(()),
            Err(IN_PROGRESS) | Err(WOULD_BLOCK) =>
                Ok(writable(self, &Timeout::default())?),
            Err(err) =>
                Err(err.into()),
        }
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self)?)
    }

    pub fn nonblocking_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            Ok(read(self, buf)?)
        }
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

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            Ok(send(self, buf, flags)?)
        }
    }

    pub fn nonblocking_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            Ok(write(self, buf)?)
        }
    }

    pub fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
    {
        Ok(getsockopt(self)?)
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
        where C: IoControl
    {
        Ok(ioctl(self, cmd)?)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0)
        }
        while !self.as_ctx().stopped() {
            match read(self, buf) {
                Ok(len) =>
                    return Ok(len),
                Err(INTERRUPTED) =>
                    (),
                Err(TRY_AGAIN) | Err(WOULD_BLOCK) =>
                    if let Err(err) = readable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) =>
                    return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0)
        }
        while !self.as_ctx().stopped() {
            match recv(self, buf, flags) {
                Ok(len) =>
                    return Ok(len),
                Err(INTERRUPTED) =>
                    (),
                Err(TRY_AGAIN) | Err(WOULD_BLOCK) =>
                    if let Err(err) = readable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) =>
                    return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0)
        }
        while !self.as_ctx().stopped() {
            match send(self, buf, flags) {
                Ok(len) =>
                    return Ok(len),
                Err(INTERRUPTED) =>
                    (),
                Err(TRY_AGAIN) | Err(WOULD_BLOCK) =>
                    if let Err(err) = writable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) =>
                    return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getpeername(self)?)
    }

    pub fn set_socket_option<C>(&self, cmd: C) -> io::Result<()>
        where C: SetSocketOption<P>
    {
        Ok(setsockopt(self, cmd)?)
    }

    pub fn shutdown(&self, how: socket_base::Shutdown) -> io::Result<()> {
        Ok(shutdown(self, how)?)
    }

    pub fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0)
        }
        while !self.as_ctx().stopped() {
            match write(self, buf) {
                Ok(len) =>
                    return Ok(len),
                Err(INTERRUPTED) =>
                    (),
                Err(TRY_AGAIN) | Err(WOULD_BLOCK) =>
                    if let Err(err) = writable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) =>
                    return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }
}

unsafe impl<P> Send for StreamSocket<P> { }

unsafe impl<P> AsIoContext for StreamSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }
}

impl<P> AsRawFd for StreamSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.as_raw_fd()
    }
}

impl<P> Socket<P> for StreamSocket<P>
    where P: Protocol,
{
    fn protocol(&self) -> &P {
        self.soc.protocol()
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        StreamSocket {
            soc: SocketImpl::new(ctx, soc, pro),
        }
    }
}

impl<P> AsyncSocket for StreamSocket<P> {
    fn add_read_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.soc.add_read_op(this, op, err)
    }

    fn add_write_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.soc.add_write_op(this, op, err)
    }

    fn next_read_op(&mut self, this: &mut ThreadIoContext) {
        self.soc.next_read_op(this)
    }

    fn next_write_op(&mut self, this: &mut ThreadIoContext) {
        self.soc.next_write_op(this)
    }
}

impl<P> io::Read for StreamSocket<P>
    where P: Protocol,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_some(buf)
    }
}

impl<P> io::Write for StreamSocket<P>
    where P: Protocol,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_some(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<P> Stream for StreamSocket<P>
    where P: Protocol,
{
    fn async_read_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncRead::new(self, buf, tx));
        rx.yield_return()
    }


    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncWrite::new(self, buf, tx));
        rx.yield_return()
    }
}
