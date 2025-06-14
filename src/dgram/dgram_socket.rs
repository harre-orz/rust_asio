use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::socket::ffi::{self, Socket};
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};
use std::cell::Cell;
use std::time::{Duration, Instant};

pub struct DgramSocket<P> {
    ctx: IoContext,
    soc: Socket,
    pro: P,
    exp: Cell<Option<Instant>>,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> Result<Self, OsError> {
        let soc = ffi::socket(pro)?;
        Ok(Self::new_priv(ctx, soc, pro))
    }

    pub(crate) fn new_priv(ctx: &IoContext, soc: Socket, pro: P) -> Self {
        DgramSocket {
            ctx: ctx.clone(),
            soc: soc,
            pro: pro,
            exp: Cell::new(None),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn bind(&self, ep: &P::Endpoint) -> Result<(), OsError> {
        ffi::bind(&self.soc, ep)?;
        Ok(())
    }

    pub fn connect(&self, ep: &P::Endpoint) -> Result<(), OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(())
    }

    pub fn close(self) -> Result<(), OsError> {
        ffi::close(self.soc)
    }

    pub fn expires_at(&self, time: Instant) {
        self.exp.set(Some(time))
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

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ffi::receive_from(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc, buf)
    }

    pub fn nb_send_to<E>(&self, buf: &[u8], ep: E) -> Result<usize, OsError>
    where
        E: AsRef<P::Endpoint>,
    {
        ffi::send_to(&self.soc, buf, ep.as_ref())
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(&self.soc, buf, &self.ctx, self.exp.get())
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ops::receive_from(&self.soc, buf, &self.ctx, self.exp.get())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.soc, buf, &self.ctx, self.exp.get())
    }

    pub fn send_to<E>(&self, buf: &[u8], ep: E) -> Result<usize, OsError>
    where
        E: AsRef<P::Endpoint>,
    {
        ops::send_to(&self.soc, buf, ep.as_ref(), &self.ctx, self.exp.get())
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

    pub fn bind(&self, ep: &P::Endpoint) -> Result<(), OsError> {
        ffi::bind(self.soc.as_socket(), ep)?;
        Ok(())
    }

    pub fn connect(&self, ep: &P::Endpoint) -> Result<(), OsError> {
        ffi::connect(self.soc.as_socket(), ep)?;
        Ok(())
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

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ffi::receive_from(self.soc.as_socket(), buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(self.soc.as_socket(), buf)
    }

    pub fn nb_send_to<E>(&self, buf: &[u8], ep: E) -> Result<usize, OsError>
    where
        E: AsRef<P::Endpoint>,
    {
        ffi::send_to(self.soc.as_socket(), buf, ep.as_ref())
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

    pub async fn async_receive_from(
        &self,
        buf: &mut [u8],
    ) -> Result<(usize, P::Endpoint), OsError> {
        ops::async_receive_from(&self.soc, buf).await
    }

    pub async fn async_sent(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_send(&self.soc, buf).await
    }

    pub async fn async_send_to<E>(&self, buf: &mut [u8], ep: E) -> Result<usize, OsError>
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
