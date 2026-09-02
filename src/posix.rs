use crate::buffer::{AsyncIoStream, IoStream};
use crate::core::IoContext;
use crate::error::OsError;
use crate::primitive::{AtomicTimeout, DurationOverflowError, Fd, Socket};
use crate::socket::AsyncSocket;
use std::os::fd::RawFd;
use std::time::Duration;

pub struct StreamDescriptor {
    ctx: IoContext,
    soc: Socket,
    ato: AtomicTimeout,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        let fd = unsafe { Fd::from_raw_fd(fd) };
        let ato = ctx.timeout();
        Self {
            ctx: ctx.clone(),
            soc: Socket(fd),
            ato: ato,
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
    }

    pub fn set_timeout(&mut self, timer: Duration) -> Result<(), DurationOverflowError> {
        self.ato.set(timer)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.nb_read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.nb_write(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.read(&self.ctx, buf, self.ato.get())
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.write(&self.ctx, buf, self.ato.get())
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

pub struct AsyncStreamDescriptor {
    inner: AsyncSocket<()>,
}

impl AsyncStreamDescriptor {
    pub fn as_ctx(&self) -> &IoContext {
        &self.inner.0.1.0
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), DurationOverflowError> {
        self.inner.0.1.2.set(timeout)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_write(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        let (_, soc, ato, _) = &self.inner.0.1;
        soc.read(self.as_ctx(), buf, ato.get())
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        let (_, soc, ato, _) = &self.inner.0.1;
        soc.write(self.as_ctx(), buf, ato.get())
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.async_read(buf).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.async_write(buf).await
    }
}

impl IoStream for AsyncStreamDescriptor {
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.write_some(buf)
    }
}

impl AsyncIoStream for AsyncStreamDescriptor {
    type Error = OsError;

    async fn async_read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.async_read_some(buf).await
    }

    async fn async_write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.async_write_some(buf).await
    }
}

impl From<StreamDescriptor> for AsyncStreamDescriptor {
    fn from(soc: StreamDescriptor) -> Self {
        Self {
            inner: AsyncSocket::new(soc.ctx, soc.soc, soc.ato, ()),
        }
    }
}
