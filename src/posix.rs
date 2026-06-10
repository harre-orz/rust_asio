use crate::buffer::IoStream;
use crate::core;
use crate::core::{Fd, Timeout};
use crate::error::{OsError, Result};
use crate::socket::{AsyncSocket, Socket};
use crate::{IoContext, socket};
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

    pub async fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.read_some(buf, self.timeout).await
    }

    pub async fn write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.write_some(buf, self.timeout).await
    }
}

pub struct StreamDescriptor {
    soc: Socket,
    timeout: Timeout,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        Self {
            soc: unsafe { Socket::from_raw_fd(ctx.clone(), Fd::new_unchecked(fd)) },
            timeout: Timeout::infinite(),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout);
    }

    pub fn into_async(self) -> AsyncStreamDescriptor {
        AsyncStreamDescriptor {
            soc: AsyncSocket::new(self.soc),
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
        socket::read_some(&self.soc, buf, self.timeout)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        socket::write_some(&self.soc, buf, self.timeout)
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
