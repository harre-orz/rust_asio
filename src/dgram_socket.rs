use crate::IoContext;
use crate::error::OsError;
use crate::exec::async_socket::AsyncSocket;
use crate::ffi::socket::Socket;
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};
use std::cell::Cell;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, OsError>;

pub struct DgramSocket<P> {
    ctx: IoContext,
    soc: Socket,
    pro: P,
    cto: Cell<Option<Instant>>,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_priv(ctx: &IoContext, soc: Socket, pro: P) -> Self {
        DgramSocket {
            ctx: ctx.clone(),
            soc: soc,
            pro: pro,
            cto: Cell::new(None),
        }
    }

    pub fn new(ctx: &IoContext, pro: P) -> Result<Self> {
        let soc = Socket::new(pro)?;
        Ok(Self::new_priv(ctx, soc, pro))
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn bind(&self, ep: &P::Endpoint) -> Result<()> {
        self.soc.bind(ep)
    }

    pub fn connect(&self, ep: &P::Endpoint) -> Result<()> {
        self.soc.nb_connect(ep)
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn expires_at(&self, time: Instant) {
        self.cto.set(Some(time))
    }

    pub fn expires_from_now(&self, time: Duration) {
        self.expires_at(Instant::now() + time)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_receive(buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc.nb_receive_from(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_send(buf)
    }

    pub fn nb_send_to<E>(&self, buf: &[u8], ep: E) -> Result<usize>
    where
        E: AsRef<P::Endpoint>,
    {
        self.soc.nb_send_to(buf, ep.as_ref())
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

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        ops::receive_from(&self.soc, buf, &self.ctx, self.cto.get())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        ops::send(&self.soc, buf, &self.ctx, self.cto.get())
    }

    pub fn send_to<E>(&self, buf: &[u8], ep: E) -> Result<usize>
    where
        E: AsRef<P::Endpoint>,
    {
        ops::send_to(&self.soc, buf, ep.as_ref(), &self.ctx, self.cto.get())
    }
}

pub struct AsyncDgramSocket<P> {
    soc: AsyncSocket,
    pro: P,
}

impl<P> AsyncDgramSocket<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        &self.soc.as_ctx()
    }

    pub fn bind(&self, ep: &P::Endpoint) -> Result<()> {
        self.soc.as_socket().bind(ep)
    }

    pub fn connect(&self, ep: &P::Endpoint) -> Result<()> {
        self.soc.as_socket().nb_connect(ep)
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

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc.as_socket().nb_receive_from(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_send(buf)
    }

    pub fn nb_send_to<E>(&self, buf: &[u8], ep: E) -> Result<usize>
    where
        E: AsRef<P::Endpoint>,
    {
        self.soc.as_socket().nb_send_to(buf, ep.as_ref())
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

    pub async fn async_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        ops::async_receive_from(&self.soc, buf).await
    }

    pub async fn async_sent(&self, buf: &mut [u8]) -> Result<usize> {
        ops::async_send(&self.soc, buf).await
    }

    pub async fn async_send_to<E>(&self, buf: &mut [u8], ep: E) -> Result<usize>
    where
        E: AsRef<P::Endpoint>,
    {
        ops::async_send_to(&self.soc, buf, ep.as_ref()).await
    }
}

impl<P> From<DgramSocket<P>> for AsyncDgramSocket<P> {
    fn from(soc: DgramSocket<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            pro: soc.pro,
        }
    }
}
