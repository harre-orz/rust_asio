use crate::ffi;
use crate::{Error, IoContext, Protocol, Result, Shutdown};
use std::io;
use std::os::fd::OwnedFd;
use std::time::Duration;

pub struct StreamSocketBuilder<P: Protocol> {
    ctx: IoContext,
    pro: P,
    conn_timeout: Duration,
}

impl<P> StreamSocketBuilder<P>
where
    P: Protocol,
{
    pub fn nb_connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>> {
        let soc = ffi::socket(self.pro)?;
        ffi::connect(&soc, ep)?;
        Ok(StreamSocket::new_priv(self.ctx, self.pro, soc))
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>> {
        let soc = ffi::socket(self.pro)?;
        let soc = loop {
            match ffi::connect(&soc, ep) {
                Ok(_) => break soc,
                Err(Error::IN_PROGRESS) | Err(Error::WOULD_BLOCK) => {
                    if let Err(err) = ffi::wait_writable(&soc, self.conn_timeout) {
                        return Err(err);
                    } else {
                        break soc;
                    }
                }
                Err(Error::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        };
        Ok(StreamSocket::new_priv(self.ctx, self.pro, soc))
    }
}

pub struct StreamSocket<P> {
    ctx: IoContext,
    pro: P,
    soc: OwnedFd,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> StreamSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> StreamSocketBuilder<P> {
        StreamSocketBuilder {
            ctx: ctx.clone(),
            pro: pro,
            conn_timeout: Duration::MAX,
        }
    }

    pub(crate) fn new_priv(ctx: IoContext, pro: P, soc: OwnedFd) -> Self {
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

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        loop {
            match ffi::write(&self.soc, buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(Error::TRY_AGAIN) | Err(Error::WOULD_BLOCK) => {
                    if let Err(err) = ffi::wait_writable(&self.soc, self.write_timeout) {
                        return Err(err);
                    }
                }
                Err(Error::INTERRUPTED) => {
                    if self.ctx.is_stopped() {
                        return Err(Error::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        loop {
            match ffi::send(&self.soc, buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(Error::TRY_AGAIN) | Err(Error::WOULD_BLOCK) => {
                    if let Err(err) = ffi::wait_writable(&self.soc, self.write_timeout) {
                        return Err(err);
                    }
                }
                Err(Error::INTERRUPTED) => {
                    if self.ctx.is_stopped() {
                        return Err(Error::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        loop {
            match ffi::read(&self.soc, buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(Error::TRY_AGAIN) | Err(Error::WOULD_BLOCK) => {
                    if let Err(err) = ffi::wait_readable(&self.soc, self.read_timeout) {
                        return Err(err);
                    }
                }
                Err(Error::INTERRUPTED) => {
                    if self.ctx.is_stopped() {
                        return Err(Error::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        loop {
            match ffi::receive(&self.soc, buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(Error::TRY_AGAIN) | Err(Error::WOULD_BLOCK) => {
                    if let Err(err) = ffi::wait_readable(&self.soc, self.read_timeout) {
                        return Err(err);
                    }
                }
                Err(Error::INTERRUPTED) => {
                    if self.ctx.is_stopped() {
                        return Err(Error::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        ffi::receive(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        ffi::send(&self.soc, buf)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        ffi::getsockname(&self.soc)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        ffi::getpeername(&self.soc)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn close(self) -> Result<()> {
        ffi::close(self.soc)
    }
}

impl<P> io::Write for StreamSocket<P>
where
    P: Protocol,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_some(buf).map_err(Into::into)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<P> io::Read for StreamSocket<P>
where
    P: Protocol,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_some(buf).map_err(Into::into)
    }
}
