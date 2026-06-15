use std::cell::Cell;
use crate::buffer::{AsyncIoStream, IoStream};
use crate::core::IoContext;
use crate::error::{OsError, Result};
use crate::primitive::{Fd, Socket, Timeout};
use crate::socket::AsyncSocket;
use std::os::fd::RawFd;
use std::time::Duration;

pub struct StreamDescriptor {
    ctx: IoContext,
    soc: Socket,
    t: Cell<Timeout>,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        let t = ctx.get_timeout();
        let fd = unsafe { Fd::from_raw_fd(fd) };
        Self {
            ctx: ctx.clone(),
            soc: Socket(fd),
            t: Cell::new(t),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.t.set(Timeout::from_duration(timeout));
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_write(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.read(&self.ctx, buf, self.t.get())
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.write(&self.ctx, buf, self.t.get())
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
    inner: AsyncSocket<Cell<Timeout>>,
}

impl AsyncStreamDescriptor {
    pub fn as_ctx(&self) -> &IoContext {
        &self.inner.as_ctx()
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.inner.as_data().set(Timeout::from_duration(timeout))
    }

    fn get_timeout(&self) -> Timeout {
        self.inner.as_data().get()
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.inner.as_socket().nb_read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.inner.as_socket().nb_write(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        let t = self.get_timeout();
        self.inner.as_socket().read(self.as_ctx(), buf, t)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        let t = self.get_timeout();
        self.inner.as_socket().write(self.as_ctx(), buf, t)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        let t = self.get_timeout();
        self.inner.async_read(buf, t).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize> {
        let t = self.get_timeout();
        self.inner.async_write(buf, t).await
    }
}

impl IoStream for AsyncStreamDescriptor {
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize> {
        self.write_some(buf)
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
            inner: AsyncSocket::new(soc.ctx, soc.soc, soc.t),
        }
    }
}
