use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, Socket, Timeout};
use crate::ops;
use crate::socket_base::Protocol;

pub trait ConnectedSocket {
    type Socket;

    fn socket(&self, soc: Socket) -> Self::Socket;
}

pub struct SocketListenerBuilder<'a, P: Protocol> {
    ctx: &'a IoContext,
    soc: Socket,
    pro: P,
    max_conns: i32,
}

impl<'a, P> SocketListenerBuilder<'a, P>
where
    P: Protocol,
{
    pub fn max_conns(mut self, max_conns: i32) -> Self {
        self.max_conns = max_conns;
        self
    }

    pub fn bind(self, ep: &P::Endpoint) -> Result<Self, OsError> {
        ffi::bind(&self.soc, ep)?;
        Ok(self)
    }

    pub fn reuse_addr(self, on: bool) -> Result<Self, OsError> {
        let on = if on { 1i32 } else { 0i32 };
        ffi::setsockopt(&self.soc, libc::SOL_SOCKET, libc::SO_REUSEADDR, on)?;
        Ok(self)
    }

    pub fn listen(self) -> Result<SocketListener<P>, OsError> {
        ffi::listen(&self.soc, self.max_conns)?;
        Ok(SocketListener {
            ctx: self.ctx.clone(),
            soc: self.soc,
            pro: self.pro,
            read_timeout: Timeout::new(),
        })
    }

    pub fn listen_async(self) -> Result<AsyncSocketListener<P>, OsError> {
        let soc = self.listen()?;
        Ok(soc.into())
    }
}

pub struct SocketListener<P> {
    ctx: IoContext,
    soc: Socket,
    pro: P,
    read_timeout: Timeout,
}

impl<P> SocketListener<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> Result<SocketListenerBuilder<P>, OsError> {
        let soc = ffi::socket(pro)?;
        Ok(SocketListenerBuilder {
            ctx,
            soc,
            pro,
            max_conns: libc::SOMAXCONN,
        })
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

    pub fn protocol(&self) -> P {
        self.pro
    }
}

impl<P> SocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket,
{
    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint), OsError> {
        let (soc, ep) = ffi::accept(&self.soc)?;
        Ok((self.socket(soc), ep))
    }

    pub fn accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint), OsError> {
        let (soc, ep) = ops::accept(&self.soc, self.read_timeout, &self.ctx)?;
        Ok((self.socket(soc), ep))
    }
}

pub struct AsyncSocketListener<P> {
    soc: AsyncSocket,
    pro: P,
    read_timeout: Timeout,
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(self.soc.as_socket())
    }

    pub fn protocol(&self) -> P {
        self.pro
    }
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket,
{
    pub fn accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint), OsError> {
        let (soc, ep) = ops::accept(self.soc.as_socket(), self.read_timeout, &self.soc.as_ctx())?;
        Ok((self.socket(soc), ep))
    }

    pub async fn async_accept(
        &self,
    ) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint), OsError> {
        let (soc, ep) = ops::async_accept(&self.soc, self.read_timeout).await?;
        Ok((self.socket(soc), ep))
    }

    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint), OsError> {
        let (soc, ep) = ffi::accept(self.soc.as_socket())?;
        Ok((self.socket(soc), ep))
    }

}

impl<P> From<SocketListener<P>> for AsyncSocketListener<P> {
    fn from(soc: SocketListener<P>) -> Self {
        let SocketListener {
            ctx,
            soc,
            pro,
            read_timeout,
        } = soc;
        Self {
            soc: AsyncSocket::new(ctx, soc),
            pro,
            read_timeout,
        }
    }
}
