use super::{AsyncIoStream, IoStream};
use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, Socket, Timeout};
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};

pub struct StreamSocketBuilder<'a, P: Protocol> {
    ctx: &'a IoContext,
    soc: Socket,
    pro: P,
    conn_timeout: Timeout,
}

impl<'a, P> StreamSocketBuilder<'a, P>
where
    P: Protocol,
{
    pub fn nb_connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>, OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(StreamSocket::new_priv(self.ctx, self.soc, self.pro))
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>, OsError> {
        let soc = ops::connect(self.soc, ep, self.conn_timeout, self.ctx)?;
        Ok(StreamSocket::new_priv(self.ctx, soc, self.pro))
    }

    pub async fn async_connect(self, ep: &P::Endpoint) -> Result<AsyncStreamSocket<P>, OsError> {
        let soc = ops::async_connect(self.soc, ep, self.conn_timeout, self.ctx).await?;
        Ok(AsyncStreamSocket {
            soc,
            pro: self.pro,
            read_timeout: Timeout::new(),
            write_timeout: Timeout::new(),
        })
    }
}

pub struct StreamSocket<P> {
    ctx: IoContext,
    soc: Socket,
    pro: P,
    read_timeout: Timeout,
    write_timeout: Timeout,
}

impl<P> StreamSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> Result<StreamSocketBuilder<P>, OsError> {
        let soc = ffi::socket(pro)?;
        Ok(StreamSocketBuilder {
            ctx,
            soc,
            pro,
            conn_timeout: Timeout::new(),
        })
    }

    pub(crate) fn new_priv(ctx: &IoContext, soc: Socket, pro: P) -> Self {
        Self {
            ctx: ctx.clone(),
            soc,
            pro,
            read_timeout: Timeout::new(),
            write_timeout: Timeout::new(),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc)
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(&self.soc, buf)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::read(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc, buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::write(&self.soc, buf)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::read_some(&self.soc, buf, self.read_timeout, &self.ctx)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(&self.soc, buf, self.read_timeout, &self.ctx)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.soc, buf, self.write_timeout, &self.ctx)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::write_some(&self.soc, buf, self.write_timeout, &self.ctx)
    }
}

impl<P> IoStream for StreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.write_some(buf)
    }
}

pub struct AsyncStreamSocket<P> {
    soc: AsyncSocket,
    pro: P,
    read_timeout: Timeout,
    write_timeout: Timeout,
}

impl<P> AsyncStreamSocket<P>
where
    P: Protocol,
{
    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc.as_socket())
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(&self.soc.as_socket(), buf)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::read(&self.soc.as_socket(), buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc.as_socket(), buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::write(&self.soc.as_socket(), buf)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::read_some(
            &self.soc.as_socket(),
            buf,
            self.read_timeout,
            &self.soc.as_ctx(),
        )
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(
            &self.soc.as_socket(),
            buf,
            self.read_timeout,
            &self.soc.as_ctx(),
        )
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc.as_socket())
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(
            &self.soc.as_socket(),
            buf,
            self.write_timeout,
            &self.soc.as_ctx(),
        )
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc.as_socket(), how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::write_some(
            &self.soc.as_socket(),
            buf,
            self.write_timeout,
            &self.soc.as_ctx(),
        )
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_read_some(&self.soc, buf, self.read_timeout).await
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_receive(&self.soc, buf, self.read_timeout).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_send(&self.soc, buf, self.write_timeout).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_write_some(&self.soc, buf, self.write_timeout).await
    }
}

impl<P> IoStream for AsyncStreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.write_some(buf)
    }
}

impl<P> AsyncIoStream for AsyncStreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    async fn async_read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.async_read_some(buf).await
    }

    async fn async_write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.async_write_some(buf).await
    }
}

impl<P> From<StreamSocket<P>> for AsyncStreamSocket<P> {
    fn from(soc: StreamSocket<P>) -> Self {
        let StreamSocket {
            ctx,
            soc,
            pro,
            read_timeout,
            write_timeout,
        } = soc;
        Self {
            soc: AsyncSocket::new(ctx, soc),
            pro,
            read_timeout,
            write_timeout,
        }
    }
}
