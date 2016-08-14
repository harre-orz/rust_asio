use std::io;
use {IoObject, IoService, IoControl, Stream, Handler};
use backbone::{IoActor, AsIoActor, RawFd, AsRawFd, ioctl, getnonblock, setnonblock};
use backbone::ops::{read, write, async_read, async_write, cancel_io};

pub struct StreamDescriptor {
    io: IoActor,
}

impl StreamDescriptor {
    pub unsafe fn new(io: &IoService, fd: RawFd) -> StreamDescriptor {
        StreamDescriptor {
            io: IoActor::new(io, fd),
        }
    }

    pub fn cancel(&self) {
        cancel_io(self)
    }

    pub fn get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    pub fn io_control<C: IoControl>(&self, cmd: &mut C) -> io::Result<()> {
        ioctl(self, cmd)
    }

    pub fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }
}

impl Stream for StreamDescriptor {
    fn async_read_some<F: Handler<Self, usize>>(&self, buf: &mut [u8], handler: F) {
        async_read(self, buf, handler)
    }

    fn async_write_some<F: Handler<Self, usize>>(&self, buf: &[u8], handler: F) {
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
        self.io.io_service()
    }
}

impl AsRawFd for StreamDescriptor {
    fn as_raw_fd(&self) -> RawFd {
        self.io.as_raw_fd()
    }
}

impl AsIoActor for StreamDescriptor {
    fn as_io_actor(&self) -> &IoActor {
        &self.io
    }
}
