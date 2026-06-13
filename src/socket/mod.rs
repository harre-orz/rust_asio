use crate::buffer::MsgBuf;
use crate::core::{Event, IoContext};
use crate::error::{OsError, Result};
use crate::primitive::{Socket, Timeout};
use crate::socket_base::{Endpoint, EndpointRef};

impl Socket {
    pub(crate) fn accept<E>(&self, ctx: &IoContext, t: Timeout) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_accept() {
                Ok(soc) => return Ok(soc),
                Err(OsError::INTERRUPTED) => {}
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match t.poll_in(self) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) fn connect<E>(&self, ctx: &IoContext, ep: &EndpointRef<E>, t: Timeout) -> Result<()>
    where
        E: Endpoint,
    {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_connect(ep) {
                Ok(_) => return Ok(()),
                Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => loop {
                    match t.poll_out(self) {
                        Ok(()) => return Ok(()),
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) fn read(&self, ctx: &IoContext, buf: &mut [u8], t: Timeout) -> Result<usize> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_read(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match t.poll_in(self) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) fn recv(&self, ctx: &IoContext, buf: &mut [u8], t: Timeout) -> Result<usize> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_recv(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match t.poll_in(self) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) fn recvfrom<E>(
        &self,
        ctx: &IoContext,
        buf: &mut [u8],
        t: Timeout,
    ) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_recvfrom(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match t.poll_in(self) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) fn recvmsg(&self, ctx: &IoContext, mbuf: &mut MsgBuf, t: Timeout) -> Result<usize> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_recvmsg(mbuf, ctx) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match t.poll_in(self) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }
    pub(crate) fn send(&self, ctx: &IoContext, buf: &[u8], t: Timeout) -> Result<usize> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_send(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match t.poll_out(self) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) fn sendmsg(
        &self,
        ctx: &IoContext,
        mbuf: &mut MsgBuf,
        timeout: Timeout,
    ) -> Result<usize> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_sendmsg(mbuf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match timeout.poll_out(self) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }

    pub fn sendto<E>(
        &self,
        ctx: &IoContext,
        buf: &[u8],
        ep: &EndpointRef<E>,
        t: Timeout,
    ) -> Result<usize>
    where
        E: Endpoint,
    {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_sendto(buf, ep) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match t.poll_out(self) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if ctx.is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) fn write(&self, ctx: &IoContext, buf: &[u8], t: Timeout) -> Result<usize> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_write(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match t.poll_out(self) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {}
                Err(err) => return Err(err),
            }
        }
    }
}

pub(crate) struct AsyncSocket {
    ctx: IoContext,
    event: Event,
}

impl Drop for AsyncSocket {
    fn drop(&mut self) {
        self.ctx.del_socket(&self.event)
    }
}

impl AsyncSocket {
    pub(crate) fn new(ctx: IoContext, soc: Socket) -> Self {
        let event = ctx.add_socket(soc);
        Self {
            ctx: ctx,
            event: event,
        }
    }

    pub(crate) const fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub(crate) const fn as_socket(&self) -> &Socket {
        self.event.as_socket()
    }
}

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
