use crate::IoContext;
use crate::buffer::IoStream;
use crate::error::{OsError, Result};
use crate::exec::AsyncSocket;
use crate::ops::{self};
use crate::socket::{Fd, Socket, Timeout};
use std::os::fd::RawFd;
use std::time::Duration;

pub struct AsyncStreamDescriptor {
    soc: AsyncSocket,
    timeout: Timeout,
}

impl AsyncStreamDescriptor {
    pub fn as_ctx(&self) -> &IoContext {
        &self.soc.as_ctx()
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout);
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().write(buf)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        ops::async_read_some(&self.soc, buf, self.timeout).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize> {
        ops::async_write_some(&self.soc, buf, self.timeout).await
    }
}

pub struct StreamDescriptor {
    soc: Socket,
    ctx: IoContext,
    timeout: Timeout,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        Self {
            soc: unsafe { Socket::from_raw_fd(Fd::new_unchecked(fd)) },
            ctx: ctx.clone(),
            timeout: Timeout::infinite(),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout);
    }

    pub fn into_async(self) -> AsyncStreamDescriptor {
        AsyncStreamDescriptor {
            soc: AsyncSocket::new(self.ctx, self.soc),
            timeout: self.timeout,
        }
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.write(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        ops::read_some(&self.ctx, &self.soc, buf, self.timeout)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        ops::write_some(&self.ctx, &self.soc, buf, self.timeout)
    }
}

impl IoStream for StreamDescriptor {
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize> {
        self.write_some(buf)
    }
}
