use crate::IoContext;
use crate::buffer::MsgBuf;
use crate::error::{OsError, Result};
use crate::primitive::{Socket, Timeout};
use crate::socket_base::{Endpoint, EndpointRef};

impl Socket {
    pub(crate) fn connect<E>(
        &self,
        ctx: &IoContext,
        ep: &EndpointRef<E>,
        timeout: Timeout,
    ) -> Result<()>
    where
        E: Endpoint,
    {
        loop {
            match self.nb_connect(ep) {
                Ok(_) => return Ok(()),
                Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                    if let Err(err) = timeout.poll_out(self) {
                        return Err(err);
                    }
                }
                Err(OsError::INTERRUPTED) => {
                    if ctx.is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) fn accept<E>(&self, ctx: &IoContext, timeout: Timeout) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        loop {
            match timeout.poll_in(self) {
                Ok(()) => loop {
                    match self.nb_accept() {
                        Ok(soc) => return Ok(soc),
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
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

    pub(crate) fn write_some(
        &self,
        ctx: &IoContext,
        buf: &[u8],
        timeout: Timeout,
    ) -> Result<usize> {
        loop {
            match timeout.poll_out(self) {
                Ok(()) => loop {
                    match self.nb_write_some(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
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

    pub(crate) fn send(&self, ctx: &IoContext, buf: &[u8], timeout: Timeout) -> Result<usize> {
        loop {
            match timeout.poll_out(self) {
                Ok(()) => loop {
                    match self.nb_send(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
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

    pub(crate) fn send_to<E>(
        &self,
        ctx: &IoContext,
        buf: &[u8],
        ep: &EndpointRef<E>,
        timeout: Timeout,
    ) -> Result<usize>
    where
        E: Endpoint,
    {
        loop {
            match timeout.poll_out(self) {
                Ok(()) => loop {
                    match self.nb_send_to(buf, ep) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
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

    pub(crate) fn send_msg(
        &self,
        ctx: &IoContext,
        mbuf: &mut MsgBuf,
        timeout: Timeout,
    ) -> Result<usize> {
        loop {
            match timeout.poll_out(self) {
                Ok(()) => loop {
                    match self.nb_send_msg(mbuf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
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

    pub(crate) fn read_some(
        &self,
        ctx: &IoContext,
        buf: &mut [u8],
        timeout: Timeout,
    ) -> Result<usize> {
        loop {
            match timeout.poll_in(self) {
                Ok(()) => loop {
                    match self.nb_read_some(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
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

    pub(crate) fn receive(
        &self,
        ctx: &IoContext,
        buf: &mut [u8],
        timeout: Timeout,
    ) -> Result<usize> {
        loop {
            match timeout.poll_in(self) {
                Ok(()) => loop {
                    match self.nb_receive(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
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

    pub(crate) fn receive_from<E>(
        &self,
        ctx: &IoContext,
        buf: &mut [u8],
        timeout: Timeout,
    ) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        loop {
            match timeout.poll_in(self) {
                Ok(()) => loop {
                    match self.nb_receive_from(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
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

    pub(crate) fn receive_msg(
        &self,
        ctx: &IoContext,
        mbuf: &mut MsgBuf,
        timeout: Timeout,
    ) -> Result<usize> {
        loop {
            match timeout.poll_in(self) {
                Ok(()) => loop {
                    match self.nb_receive_msg(mbuf, ctx) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
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
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::AsyncSocket;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::AsyncSocket;
