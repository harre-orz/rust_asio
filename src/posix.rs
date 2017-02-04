use prelude::IoControl;
use ffi::{RawFd, AsRawFd, ioctl};
use core::{IoContext, AsIoContext, AsyncFd};
use async::Handler;
use streams::Stream;
use reactive_io::{AsAsyncFd, read, async_read, write, async_write, cancel,
                  getnonblock, setnonblock};

use std::io;

/// Typedef for the typical usage of a stream-oriented descriptor.
pub struct StreamDescriptor {
    fd: AsyncFd,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> StreamDescriptor {
        StreamDescriptor {
            fd: AsyncFd::new::<Self>(fd, ctx),
        }
    }

    pub fn cancel(&self) {
        cancel(self)
    }

    pub fn get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
        where C: IoControl,
    {
        ioctl(self, cmd)
    }

    pub fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }
}

impl Stream<io::Error> for StreamDescriptor {
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>,
    {
        async_read(self, buf, handler)
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        async_write(self, buf, handler)
    }

    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        read(self, buf)
    }

    fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        write(self, buf)
    }
}

impl AsRawFd for StreamDescriptor {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

unsafe impl Send for StreamDescriptor {
}

unsafe impl AsIoContext for StreamDescriptor {
    fn as_ctx(&self) -> &IoContext {
        self.fd.as_ctx()
    }
}

impl AsAsyncFd for StreamDescriptor {
    fn as_fd(&self) -> &AsyncFd {
        &self.fd
    }
}
