use crate::buffer::MsgBuf;
use crate::error::{OsError, Result};
use crate::IoContext;
use crate::primitive::Timeout;
use crate::socket_base::{Endpoint, EndpointRef};
use crate::primitive::Socket;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::{AsyncSocket};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::{AsyncSocket};

impl Socket {
    pub fn connect<E>(&self, ctx: &IoContext, ep: &EndpointRef<E>, timeout: Timeout) -> Result<()>
    where
        E: Endpoint,
    {
        loop {
            match self.nb_connect(ep) {
                Ok(_) => return Ok(()),
                Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                    if let Err(err) = self.poll_out(timeout) {
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

    pub fn accept<E>(&self, ctx: &IoContext, timeout: Timeout) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        loop {
            match self.poll_in(timeout) {
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

    pub fn write_some(&self, ctx: &IoContext, buf: &[u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_out(timeout) {
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

    pub fn send(&self, ctx: &IoContext, buf: &[u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_out(timeout) {
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

    pub fn send_to<E>(&self, ctx: &IoContext, buf: &[u8], ep: &EndpointRef<E>, timeout: Timeout) -> Result<usize>
    where
        E: Endpoint,
    {
        loop {
            match self.poll_out(timeout) {
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

    pub fn send_msg(&self, ctx: &IoContext, mbuf: &mut MsgBuf, timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_out(timeout) {
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

    pub fn read_some(&self, ctx: &IoContext, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_in(timeout) {
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

    pub fn receive(&self, ctx: &IoContext, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_in(timeout) {
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

    pub fn receive_from<E>(&self, ctx: &IoContext, buf: &mut [u8], timeout: Timeout) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        loop {
            match self.poll_in(timeout) {
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

    pub fn receive_msg(&self, ctx: &IoContext, mbuf: &mut MsgBuf, timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_in(timeout) {
                Ok(()) => loop {
                    match self.nb_receive_msg(mbuf) {
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
