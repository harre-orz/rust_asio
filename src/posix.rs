use ffi::{AsRawFd, RawFd, SystemError, Timeout, ioctl};
use reactor::{SocketImpl};
use core::{IoControl, AsIoContext, IoContext, Perform, ThreadIoContext, Cancel};
use handler::{Handler, AsyncReadOp, AsyncWriteOp};
use read_ops::{Read, async_read_op, blocking_read_op, nonblocking_read_op};
use write_ops::{Write, async_write_op, blocking_write_op, nonblocking_write_op};
use stream::Stream;

use std::io;
use std::time::Duration;

/// Typedef for the typical usage of a stream-oriented descriptor.
pub struct StreamDescriptor {
    pimpl: Box<SocketImpl<()>>,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        StreamDescriptor { pimpl: SocketImpl::new(ctx, fd, ()) }
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
    where
        C: IoControl,
    {
        Ok(ioctl(self, cmd)?)
    }

    pub fn nonblocking_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        nonblocking_read_op(self, buf, Read::new())
    }

    pub fn nonblocking_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        nonblocking_write_op(self, buf, Write::new())
    }

    pub fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        blocking_read_op(self, buf, &self.pimpl.timeout, Read::new())
    }

    pub fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        blocking_write_op(self, buf, &self.pimpl.timeout, Write::new())
    }

    pub fn get_timeout(&self) -> Duration {
        self.pimpl.timeout.get()
    }

    pub fn set_timeout(&self, timeout: Duration) -> io::Result<()> {
        Ok(self.pimpl.timeout.set(timeout)?)
    }
}

unsafe impl AsIoContext for StreamDescriptor {
    fn as_ctx(&self) -> &IoContext {
        self.pimpl.as_ctx()
    }
}

impl AsRawFd for StreamDescriptor {
    fn as_raw_fd(&self) -> RawFd {
        self.pimpl.as_raw_fd()
    }
}

impl Cancel for StreamDescriptor {
    fn cancel(&self) {
        self.pimpl.cancel()
    }
}

impl AsyncReadOp for StreamDescriptor {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.pimpl.add_read_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.pimpl.next_read_op(this)
    }
}


impl AsyncWriteOp for StreamDescriptor {
    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.pimpl.add_write_op(this, op, err)
    }

    fn next_write_op(&self, this: &mut ThreadIoContext) {
        self.pimpl.next_write_op(this)
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
        async_read_op(self, buf, handler, Read::new())
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        async_write_op(self, buf, handler, Write::new())
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
