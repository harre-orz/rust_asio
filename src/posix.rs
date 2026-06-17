use crate::buffer::{AsyncIoStream, IoStream};
use crate::core::IoContext;
use crate::error::OsError;
use crate::primitive::{AtomicTimeout, Fd, Socket, Timeout, TimeoutError};
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

    pub fn set_timeout(&mut self, timer: Duration) -> Result<(), TimeoutError> {
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
    inner: AsyncSocket<AtomicTimeout>,
}

impl AsyncStreamDescriptor {
    pub fn as_ctx(&self) -> &IoContext {
        &self.inner.as_ctx()
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        //self.inner.as_data().set(Timeout::from_duration(timeout))
    }

    fn timeout(&self) -> Timeout {
        self.inner.as_data().get()
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.as_socket().nb_read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.as_socket().nb_write(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner
            .as_socket()
            .read(self.as_ctx(), buf, self.timeout())
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner
            .as_socket()
            .write(self.as_ctx(), buf, self.timeout())
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.async_read(buf, self.timeout()).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.async_write(buf, self.timeout()).await
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
            inner: AsyncSocket::new(soc.ctx, soc.soc, soc.ato),
        }
    }
}
