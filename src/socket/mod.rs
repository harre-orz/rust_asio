use crate::buffer::MsgBuf;
use crate::core::{Event, IoContext};
use crate::error::OsError;
use crate::primitive::{AtomicTimeout, Socket, Timeout};
use crate::socket_base::{Endpoint, EndpointRef};
use std::pin::Pin;

impl Socket {
    pub(crate) fn accept<E>(&self, ctx: &IoContext, t: Timeout) -> Result<(Socket, E), OsError>
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
                    match self.poll_in(t) {
                        Ok(()) => break,
                        Err(OsError::INTERRUPTED) => {}
                        Err(err) => return Err(err),
                    }
                },
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) fn connect<E>(
        &self,
        ctx: &IoContext,
        ep: &EndpointRef<E>,
        t: Timeout,
    ) -> Result<(), OsError>
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
                    match self.poll_out(t) {
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

    pub(crate) fn read(
        &self,
        ctx: &IoContext,
        buf: &mut [u8],
        t: Timeout,
    ) -> Result<usize, OsError> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_read(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match self.poll_in(t) {
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

    pub(crate) fn recv(
        &self,
        ctx: &IoContext,
        buf: &mut [u8],
        t: Timeout,
    ) -> Result<usize, OsError> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_recv(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match self.poll_in(t) {
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
    ) -> Result<(usize, E), OsError>
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
                    match self.poll_in(t) {
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

    pub(crate) fn recvmsg(
        &self,
        ctx: &IoContext,
        msg: &mut MsgBuf,
        t: Timeout,
    ) -> Result<usize, OsError> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_recvmsg(msg, ctx) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match self.poll_in(t) {
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
    pub(crate) fn send(&self, ctx: &IoContext, buf: &[u8], t: Timeout) -> Result<usize, OsError> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_send(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match self.poll_out(t) {
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
        msg: &mut MsgBuf,
        t: Timeout,
    ) -> Result<usize, OsError> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_sendmsg(msg) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match self.poll_out(t) {
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

    pub(crate) fn sendto<E>(
        &self,
        ctx: &IoContext,
        buf: &[u8],
        ep: &EndpointRef<E>,
        t: Timeout,
    ) -> Result<usize, OsError>
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
                    match self.poll_out(t) {
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

    pub(crate) fn write(&self, ctx: &IoContext, buf: &[u8], t: Timeout) -> Result<usize, OsError> {
        if ctx.is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        loop {
            match self.nb_write(buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => loop {
                    match self.poll_out(t) {
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

pub(crate) struct AsyncSocket<T>(
    pub(crate) Pin<Box<(Event, (IoContext, Socket, AtomicTimeout, T))>>,
);

impl<T> Drop for AsyncSocket<T> {
    fn drop(&mut self) {
        let (ctx, soc, _, _) = &self.0.1;
        ctx.inner.reactor.del_socket(soc);
    }
}

impl<T> AsyncSocket<T> {
    pub(crate) fn new(ctx: IoContext, soc: Socket, ato: AtomicTimeout, data: T) -> Self {
        let ev = Event::new((ctx, soc, ato, data));
        ev.1.0.inner.reactor.add_socket(&ev.1.1, &ev.0);
        Self(ev)
    }
}

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
