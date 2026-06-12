use crate::buffer::{AsyncIoStream, IoStream};
use crate::core::IoContext;
use crate::error::{OsError, Result};
use crate::primitive::Timeout;
use crate::primitive::{Fd, Socket};
use crate::socket::AsyncSocket;
use std::os::fd::RawFd;
use std::time::Duration;

pub struct StreamDescriptor {
    ctx: IoContext,
    soc: Socket,
    timeout: Timeout,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        let fd = Fd::from_raw_fd(fd);
        Self {
            ctx: ctx.clone(),
            soc: unsafe { Socket(fd) },
            timeout: Timeout::INFINITE,
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

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_read_some(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_write_some(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.read_some(&self.ctx, buf, self.timeout)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.write_some(&self.ctx, buf, self.timeout)
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
        self.soc.as_socket().nb_read_some(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_write_some(buf)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.async_read_some(buf, self.timeout).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.async_write_some(buf, self.timeout).await
    }
}

impl AsyncIoStream for AsyncStreamDescriptor {
    type Error = OsError;

    async fn async_read(&self, buf: &mut [u8]) -> Result<usize> {
        self.async_read_some(buf).await
    }

    async fn async_write(&self, buf: &[u8]) -> Result<usize> {
        self.async_write_some(buf).await
    }
}

impl From<StreamDescriptor> for AsyncStreamDescriptor {
    fn from(soc: StreamDescriptor) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            timeout: soc.timeout,
        }
    }
}
