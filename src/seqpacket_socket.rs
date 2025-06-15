use crate::IoContext;
use crate::error::OsError;
use crate::exec::async_socket::AsyncSocket;
use crate::ffi::socket::Socket;
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};
use std::cell::Cell;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, OsError>;

pub struct SeqPacketSocketBuilder<'a, P: Protocol> {
    ctx: &'a IoContext,
    soc: Socket,
    pro: P,
}

impl<'a, P> SeqPacketSocketBuilder<'a, P>
where
    P: Protocol,
{
    pub fn connect(self, ep: &P::Endpoint) -> Result<SeqPacketSocket<P>> {
        self.soc.nb_connect(ep)?;
        Ok(SeqPacketSocket::new_priv(self.ctx, self.soc, self.pro))
    }

    pub fn connect_async(self, ep: &P::Endpoint) -> Result<AsyncSeqPacketSocket<P>> {
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
    pub fn new(ctx: &IoContext, pro: P) -> Result<SeqPacketSocketBuilder<P>> {
        let soc = Socket::new(pro)?;
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

    pub fn expires_at(&self, cto: Instant) {
        self.cto.set(Some(cto))
    }

    pub fn expires_from_now(&self, cto: Duration) {
        self.expires_at(Instant::now() + cto)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_receive(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_send(buf)
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        ops::receive(&self.soc, buf, &self.ctx, self.cto.get())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
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

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().nb_receive(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_send(buf)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.as_socket().shutdown(how)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getpeername()
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize> {
        ops::async_receive(&self.soc, buf).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize> {
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
