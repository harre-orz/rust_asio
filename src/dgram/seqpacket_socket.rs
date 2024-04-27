use crate::error::OsError;
use crate::ffi::{self, ConnectedSocket};
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};
use crate::IoContext;
use std::time::Duration;

pub struct SeqPacketSocketBuilder<P: Protocol> {
    ctx: IoContext,
    pro: P,
    ep: Option<P::Endpoint>,
}

impl<P> SeqPacketSocketBuilder<P>
where
    P: Protocol,
{
    pub fn bind(mut self, ep: P::Endpoint) -> Self {
        self.ep = Some(ep);
        self
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<SeqPacketSocket<P>, OsError> {
        let soc = ffi::socket(self.pro)?;
        if let Some(ep) = self.ep {
            ffi::bind(&soc, &ep)?;
        }
        ffi::connect(&soc, ep)?;
        Ok(SeqPacketSocket::new_priv(self.ctx, self.pro, soc))
    }

    pub fn listen(self) -> Result<SeqPacketSocket<P>, OsError> {
        let soc = ffi::socket(self.pro)?;
        if let Some(ep) = self.ep {
            ffi::bind(&soc, &ep)?;
        }
        Ok(SeqPacketSocket::new_priv(self.ctx, self.pro, soc))
    }
}

pub struct SeqPacketSocket<P> {
    ctx: IoContext,
    pro: P,
    soc: ConnectedSocket,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> SeqPacketSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> SeqPacketSocketBuilder<P> {
        SeqPacketSocketBuilder {
            ctx: ctx.clone(),
            pro: pro,
            ep: None,
        }
    }

    pub(crate) fn new_priv(ctx: IoContext, pro: P, soc: ConnectedSocket) -> Self {
        Self {
            ctx: ctx,
            pro: pro,
            soc: soc,
            read_timeout: Duration::MAX,
            write_timeout: Duration::MAX,
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    // pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
    //     ops::async_receive(&self.ctx, &self.soc, buf, self.read_timeout).await
    // }
    //
    // pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
    //     ops::async_send(&self.ctx, &self.soc, buf, self.write_timeout).await
    // }

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
        ops::receive(&self.ctx, &self.soc, buf, self.read_timeout)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.ctx, &self.soc, buf, self.write_timeout)
    }
}

pub struct AsyncSeqPacketSocket<P> {
    inner: Box<SeqPacketSocket<P>>,
}

impl<P> Drop for AsyncSeqPacketSocket<P> {
    fn drop(&mut self) {}
}

impl<P> AsyncSeqPacketSocket<P> {}
