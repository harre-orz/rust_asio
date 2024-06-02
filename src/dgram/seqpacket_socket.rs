use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, Socket, Timeout};
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};

pub struct SeqPacketSocketBuilder<'a, P: Protocol> {
    ctx: &'a IoContext,
    soc: Socket,
    pro: P,
}

impl<'a, P> SeqPacketSocketBuilder<'a, P>
where
    P: Protocol,
{
    pub fn connect(self, ep: &P::Endpoint) -> Result<SeqPacketSocket<P>, OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(SeqPacketSocket::new_priv(self.ctx, self.soc, self.pro))
    }

    pub fn connect_async(self, ep: &P::Endpoint) -> Result<AsyncSeqPacketSocket<P>, OsError> {
        let soc = self.connect(ep)?;
        Ok(soc.into())
    }
}

pub struct SeqPacketSocket<P> {
    ctx: IoContext,
    soc: Socket,
    pro: P,
    read_timeout: Timeout,
    write_timeout: Timeout,
}

impl<P> SeqPacketSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> Result<SeqPacketSocketBuilder<P>, OsError> {
        let soc = ffi::socket(pro)?;
        Ok(SeqPacketSocketBuilder { ctx, soc, pro })
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

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc)
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc, buf)
    }

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
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
}

pub struct AsyncSeqPacketSocket<P> {
    soc: AsyncSocket,
    pro: P,
    read_timeout: Timeout,
    write_timeout: Timeout,
}

impl<P> AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        &self.soc.as_ctx()
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(self.soc.as_socket())
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(self.soc.as_socket(), buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(self.soc.as_socket(), buf)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(self.soc.as_socket(), how)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(
            self.soc.as_socket(),
            buf,
            self.read_timeout,
            self.soc.as_ctx(),
        )
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(self.soc.as_socket())
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(
            self.soc.as_socket(),
            buf,
            self.write_timeout,
            self.soc.as_ctx(),
        )
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_receive(&self.soc, buf, self.read_timeout).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_send(&self.soc, buf, self.write_timeout).await
    }
}

impl<P> From<SeqPacketSocket<P>> for AsyncSeqPacketSocket<P> {
    fn from(soc: SeqPacketSocket<P>) -> Self {
        let SeqPacketSocket {
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
