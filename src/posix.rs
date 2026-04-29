use crate::IoContext;
use crate::buffer::IoStream;
use crate::error::OsError;
use crate::exec::AsyncSocket;
use crate::ops::{self, Blocking};
use crate::socket::{Fd, Socket};
use std::os::fd::RawFd;
use std::time::{Duration, Instant};

pub struct AsyncStreamDescriptor {
    soc: AsyncSocket,
}

impl AsyncStreamDescriptor {
    pub fn as_ctx(&self) -> &IoContext {
        &self.soc.as_ctx()
    }

    pub fn expires_at(&self, timeout: Instant) {
        self.soc.update_schedule(timeout)
    }

    pub fn expires_from_now(&self, timeout: Duration) {
        self.expires_at(Instant::now() + timeout)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.as_socket().read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.as_socket().write(buf)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_read_some(&self.soc, buf).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_write_some(&self.soc, buf).await
    }
}

pub struct StreamDescriptor {
    blk: Blocking,
    soc: Socket,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        Self {
            blk: Blocking::new(ctx.clone()),
            soc: unsafe { Socket::from_raw_fd(Fd::new_unchecked(fd)) },
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.blk.as_ctx()
    }

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
    }

    pub fn into_async(self) -> AsyncStreamDescriptor {
        AsyncStreamDescriptor {
            soc: AsyncSocket::new(self.blk.into_ctx(), self.soc),
        }
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.write(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::read_some(&self.soc, buf, &self.blk)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::write_some(&self.soc, buf, &self.blk)
    }
}

impl IoStream for StreamDescriptor {
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.write_some(buf)
    }
}
