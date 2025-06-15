use crate::IoContext;
use crate::error::OsError;
use crate::exec::async_socket::AsyncSocket;
use crate::ffi::socket::{Fd, Socket};
use crate::io_stream::IoStream;
use crate::ops;
use std::cell::Cell;
use std::os::fd::RawFd;
use std::time::Instant;

pub struct StreamDescriptor {
    ctx: IoContext,
    soc: Socket,
    cto: Cell<Option<Instant>>,
}

impl StreamDescriptor {
    pub unsafe fn from_raw_fd(ctx: &IoContext, fd: RawFd) -> Self {
        Self {
            ctx: ctx.clone(),
            soc: unsafe { Socket::from_raw_fd(Fd::new_unchecked(fd)) },
            cto: Cell::new(None),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.nb_read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.nb_write(buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::read_some(&self.soc, buf, &self.ctx, self.cto.get())
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::write_some(&self.soc, buf, &self.ctx, self.cto.get())
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
    soc: AsyncSocket,
}

impl AsyncStreamDescriptor {
    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.as_socket().nb_read(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.as_socket().nb_write(buf)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_read_some(&self.soc, buf).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_write_some(&self.soc, buf).await
    }
}

impl From<StreamDescriptor> for AsyncStreamDescriptor {
    fn from(soc: StreamDescriptor) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc),
        }
    }
}
