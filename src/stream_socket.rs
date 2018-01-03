use prelude::*;
use ffi::*;
use core::{IoContext, AsIoContext, ThreadIoContext, Perform, Yield, SocketImpl, AsyncSocket};
use async::{Handler, AsyncConnect, AsyncRead, AsyncRecv, AsyncSend, AsyncWrite};
use streams::Stream;
use socket_base;

use std::io;


pub struct StreamSocket<P> {
    soc: Box<(SocketImpl, P)>,
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
        rx.yield_return(self.as_ctx())
    }

    pub fn async_recv<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncRecv::new(self, buf, flags, tx));
        rx.yield_return(self.as_ctx())
    }

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncSend::new(self, buf, flags, tx));
        rx.yield_return(self.as_ctx())
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = socket_base::BytesReadable::default();
        ioctl(self, &mut bytes)?;
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep).map_err(From::from)
    }

    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        if self.as_ctx().stopped() {
            return Err(OPERATION_CANCELED.into())
        }

        match connect(self, ep) {
            Ok(_) => Ok(()),
            Err(IN_PROGRESS) =>
                match writable(self, &Timeout::default()) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err.into()),
                },
            Err(err) => Err(err.into()),
        }
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self).map_err(From::from)
    }

        pub fn nonblocking_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            read(self, buf).map_err(From::from)
        }
    }

    pub fn nonblocking_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            recv(self, buf, flags).map_err(From::from)
        }
    }

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            send(self, buf, flags).map_err(From::from)
        }
    }

    pub fn nonblocking_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            write(self, buf).map_err(From::from)
        }
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

    pub fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0)
        }

        while !self.as_ctx().stopped() {
            match read(self, buf) {
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) | Err(WOULD_BLOCK) =>
                    if let Err(err) = readable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) => return Err(err.into()),
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
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) | Err(WOULD_BLOCK) =>
                    if let Err(err) = readable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) => return Err(err.into()),
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
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) | Err(WOULD_BLOCK) =>
                    if let Err(err) = writable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self).map_err(From::from)
    }

    pub fn set_socket_option<C>(&self, cmd: C) -> io::Result<()>
        where C: SetSocketOption<P>
    {
        setsockopt(self, cmd).map_err(From::from)
    }

    pub fn shutdown(&self, how: socket_base::Shutdown) -> io::Result<()> {
        shutdown(self, how).map_err(From::from)
    }

    pub fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0)
        }

        while !self.as_ctx().stopped() {
            match write(self, buf) {
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) | Err(WOULD_BLOCK) =>
                    if let Err(err) = writable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }
}

unsafe impl<P> Send for StreamSocket<P> { }

unsafe impl<P> AsIoContext for StreamSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        self.soc.0.as_ctx()
    }
}

impl<P> AsRawFd for StreamSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.0.as_raw_fd()
    }
}

impl<P> Socket<P> for StreamSocket<P>
    where P: Protocol,
{
    fn protocol(&self) -> &P {
        &self.soc.1
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        StreamSocket {
            soc: box (SocketImpl::new(ctx, soc), pro)
        }
    }
}

impl<P> AsyncSocket for StreamSocket<P> {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.soc.0.add_read_op(this, op, err)
    }

    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.soc.0.add_write_op(this, op, err)
    }

    fn cancel_read_ops(&self, this: &mut ThreadIoContext) {
        self.soc.0.cancel_read_ops(this)
    }

    fn cancel_write_ops(&self, this: &mut ThreadIoContext) {
        self.soc.0.cancel_write_ops(this)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.soc.0.next_read_op(this)
    }

    fn next_write_op(&self, this: &mut ThreadIoContext) {
        self.soc.0.next_write_op(this)
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

impl<P> Stream<P> for StreamSocket<P>
    where P: Protocol,
{
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncRead::new(self, buf, tx));
        rx.yield_return(self.as_ctx())
    }


    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncWrite::new(self, buf, tx));
        rx.yield_return(self.as_ctx())
    }
}
