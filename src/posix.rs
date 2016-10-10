use std::io;
use traits::{IoControl};
use stream::Stream;
use io_service::{IoObject, IoService, Handler, IoActor};
use fd_ops::*;

pub struct StreamDescriptor {
    act: IoActor,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(io: &IoService, fd: RawFd) -> StreamDescriptor {
        StreamDescriptor {
            act: IoActor::new(io, fd),
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

impl Stream for StreamDescriptor {
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize>,
    {
        async_read(self, buf, handler)
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize>
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

impl IoObject for StreamDescriptor {
    fn io_service(&self) -> &IoService {
        self.act.io_service()
    }
}

impl AsRawFd for StreamDescriptor {
    fn as_raw_fd(&self) -> RawFd {
        self.act.as_raw_fd()
    }
}

impl AsIoActor for StreamDescriptor {
    fn as_io_actor(&self) -> &IoActor {
        &self.act
    }
}
