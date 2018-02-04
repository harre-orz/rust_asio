#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, InnerSocket, IoContext, Perform, ThreadIoContext};
use handler::Handler;
use ops::*;
use streams::Stream;

use std::io;

/// Typedef for the typical usage of a stream-oriented descriptor.
pub struct StreamDescriptor {
    inner: Box<InnerSocket<()>>,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        StreamDescriptor {
            inner: InnerSocket::new(ctx, fd, ()),
        }
    }

    pub fn cancel(&self) {
        self.inner.cancel()
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
    where
        C: IoControl,
    {
        Ok(ioctl(self, cmd)?)
    }

    pub fn nonblocking_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        nonblocking_read(self, buf)
    }

    pub fn nonblocking_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        nonblocking_write(self, buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        read_timeout(self, buf, &Timeout::default())
    }

    pub fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        write_timeout(self, buf, &Timeout::default())
    }
}

unsafe impl AsIoContext for StreamDescriptor {
    fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }
}

impl AsRawFd for StreamDescriptor {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl AsyncReadOp for StreamDescriptor {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.inner.add_read_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.inner.next_read_op(this)
    }
}


impl AsyncWriteOp for StreamDescriptor {
    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.inner.add_write_op(this, op, err)
    }

    fn next_write_op(&self, this: &mut ThreadIoContext) {
        self.inner.next_write_op(this)
    }
}

impl io::Read for StreamDescriptor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_some(buf)
    }
}

unsafe impl Send for StreamDescriptor {}

impl Stream for StreamDescriptor {
    type Error = io::Error;

    fn async_read_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        async_read(self, buf, handler)
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        async_write(self, buf, handler)
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
