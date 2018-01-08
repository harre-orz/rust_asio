use prelude::*;
use ffi::*;
use core::{IoContext, AsIoContext, ThreadIoContext, Perform, SocketImpl, AsyncSocket};
use async::{Handler, Yield, AsyncRead, AsyncWrite};
use streams::Stream;

use std::io;


/// Typedef for the typical usage of a stream-oriented descriptor.
pub struct StreamDescriptor {
    soc: SocketImpl<()>,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        StreamDescriptor {
            soc: SocketImpl::new(ctx, fd, ()),
        }
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
        where C: IoControl
    {
        Ok(ioctl(self, cmd)?)
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

    pub fn nonblocking_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            Ok(write(self, buf)?)
        }
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

unsafe impl Send for StreamDescriptor {}

impl AsRawFd for StreamDescriptor {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.as_raw_fd()
    }
}

impl io::Read for StreamDescriptor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_some(buf)
    }
}


impl io::Write for StreamDescriptor {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_some(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

unsafe impl AsIoContext for StreamDescriptor {
    fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }
}

impl Stream for StreamDescriptor
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

impl AsyncSocket for StreamDescriptor {
    fn add_read_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.soc.add_read_op(this, op, err)
    }

    fn add_write_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.soc.add_write_op(this, op, err)
    }

    fn cancel_ops(&mut self, ctx: &IoContext) {
        self.soc.cancel_ops(ctx)
    }

    fn next_read_op(&mut self, this: &mut ThreadIoContext) {
        self.soc.next_read_op(this)
    }

    fn next_write_op(&mut self, this: &mut ThreadIoContext) {
        self.soc.next_write_op(this)
    }
}
