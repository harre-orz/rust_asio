use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ops;
use crate::socket::ffi::{self, Socket};
use crate::socket_base::{Protocol, Shutdown};
use std::cell::Cell;
use std::time::{Duration, Instant};

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
    cto: Cell<Option<Instant>>,
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
            soc: soc,
            pro: pro,
            cto: Cell::new(None),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn expires_at(&self, time: Instant) {
        self.cto.set(Some(time))
    }

    pub fn expires_from_now(&self, time: Duration) {
        self.expires_at(Instant::now() + time)
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
        ffi::close(self.soc)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(&self.soc, buf, &self.ctx, self.cto.get())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.soc, buf, &self.ctx, self.cto.get())
    }
}

pub struct AsyncSeqPacketSocket<P> {
    soc: AsyncSocket,
    pro: P,
}

impl<P> AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        &self.soc.as_ctx()
    }

    pub fn expires_at(&self, time: Instant) {
        self.soc.update_schedule(time)
    }

    pub fn expires_from_now(&self, time: Duration) {
        self.expires_at(Instant::now() + time)
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

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(self.soc.as_socket())
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_receive(&self.soc, buf).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_send(&self.soc, buf).await
    }
}

impl<P> From<SeqPacketSocket<P>> for AsyncSeqPacketSocket<P> {
    fn from(soc: SeqPacketSocket<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            pro: soc.pro,
        }
    }
}
