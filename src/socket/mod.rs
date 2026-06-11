use crate::buffer::MsgBuf;
use crate::error::{OsError, Result};
use crate::poll::Timeout;
use crate::socket_base::{Endpoint, EndpointRef};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::{AsyncSocket, Socket};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::{AsyncSocket, Socket};

impl Socket {
    pub fn connect<E>(&self, ep: &EndpointRef<E>, timeout: Timeout) -> Result<()>
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
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn accept<E>(&self, timeout: Timeout) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        loop {
            match self.poll_in(timeout) {
                Ok(()) => loop {
                    match self.nb_accept() {
                        Ok(soc) => return Ok(soc),
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn write_some(&self, buf: &[u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_out(timeout) {
                Ok(()) => loop {
                    match self.nb_write_some(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn send(&self, buf: &[u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_out(timeout) {
                Ok(()) => loop {
                    match self.nb_send(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn send_to<E>(&self, buf: &[u8], ep: &EndpointRef<E>, timeout: Timeout) -> Result<usize>
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
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf, timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_out(timeout) {
                Ok(()) => loop {
                    match self.nb_send_msg(mbuf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn read_some(&self, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_in(timeout) {
                Ok(()) => loop {
                    match self.nb_read_some(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn receive(&self, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_in(timeout) {
                Ok(()) => loop {
                    match self.nb_receive(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn receive_from<E>(&self, buf: &mut [u8], timeout: Timeout) -> Result<(usize, E)>
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
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf, timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_in(timeout) {
                Ok(()) => loop {
                    match self.nb_receive_msg(mbuf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}
