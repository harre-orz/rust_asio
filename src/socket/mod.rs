use crate::buffer::MsgBuf;
use crate::poll::{IoContext, Event, Timeout};
use crate::error::{OsError, Result};
use crate::socket_base::{Endpoint, EndpointRef};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::{AsyncSocket, Socket};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::socket_windows::{AsyncSocket, Socket};

pub(crate) fn connect<E>(soc: &Socket, ep: &EndpointRef<E>, timeout: Timeout) -> Result<()>
where
    E: Endpoint,
{
    loop {
        match soc.connect(ep) {
            Ok(_) => return Ok(()),
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.poll_out(timeout) {
                    return Err(err);
                }
            }
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn accept<E>(soc: &Socket, timeout: Timeout) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.accept() {
                    Ok(soc) => return Ok(soc),
                    Err(OsError::INTERRUPTED) => {
                        if soc.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn write_some(soc: &Socket, buf: &[u8], timeout: Timeout) -> Result<usize> {
    loop {
        match soc.poll_out(timeout) {
            Ok(()) => loop {
                match soc.write(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if soc.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn send(soc: &Socket, buf: &[u8], timeout: Timeout) -> Result<usize> {
    loop {
        match soc.poll_out(timeout) {
            Ok(()) => loop {
                match soc.send(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if soc.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn send_to<E>(
    soc: &Socket,
    buf: &[u8],
    ep: &EndpointRef<E>,

    timeout: Timeout,
) -> Result<usize>
where
    E: Endpoint,
{
    loop {
        match soc.poll_out(timeout) {
            Ok(()) => loop {
                match soc.send_to(buf, ep) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if soc.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn send_msg(soc: &Socket, mbuf: &mut MsgBuf, timeout: Timeout) -> Result<usize> {
    loop {
        match soc.poll_out(timeout) {
            Ok(()) => loop {
                match soc.send_msg(mbuf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if soc.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn read_some(soc: &Socket, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.read(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if soc.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn receive(soc: &Socket, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.receive(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if soc.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn receive_from<E>(soc: &Socket, buf: &mut [u8], timeout: Timeout) -> Result<(usize, E)>
where
    E: Endpoint,
{
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.receive_from(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if soc.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn receive_msg(soc: &Socket, mbuf: &mut MsgBuf, timeout: Timeout) -> Result<usize> {
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.receive_msg(mbuf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if soc.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}
