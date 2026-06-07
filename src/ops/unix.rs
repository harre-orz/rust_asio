use crate::buffer::MsgBuf;
use crate::error::{OsError, Result};
use crate::exec::AsyncSocket;
use crate::socket::{Socket, Timeout};
use crate::socket_base::{Endpoint, EndpointRef};

pub(crate) async fn async_accept<E>(soc: &AsyncSocket, timeout: Timeout) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    loop {
        match soc.poll_in(timeout).await {
            Ok(()) => loop {
                match soc.as_socket().accept() {
                    Ok(soc) => return Ok(soc),
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

pub(crate) async fn async_connect<E>(
    soc: &AsyncSocket,
    ep: &EndpointRef<'_, E>,
    timeout: Timeout,
) -> Result<()>
where
    E: Endpoint,
{
    loop {
        match soc.as_socket().connect(ep) {
            Ok(_) => return Ok(()),
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.poll_out(timeout).await {
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

pub(crate) async fn async_write_some(
    soc: &AsyncSocket,
    buf: &[u8],
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_out(timeout).await {
            Ok(()) => loop {
                match soc.as_socket().write(buf) {
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

pub(crate) async fn async_send(soc: &AsyncSocket, buf: &[u8], timeout: Timeout) -> Result<usize> {
    loop {
        match soc.poll_out(timeout).await {
            Ok(()) => loop {
                match soc.as_socket().send(buf) {
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

pub(crate) async fn async_send_to<E>(
    soc: &AsyncSocket,
    buf: &[u8],
    ep: &EndpointRef<'_, E>,
    timeout: Timeout,
) -> Result<usize>
where
    E: Endpoint,
{
    loop {
        match soc.poll_out(timeout).await {
            Ok(()) => loop {
                match soc.as_socket().send_to(buf, ep) {
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

pub(crate) async fn async_send_msg(
    soc: &AsyncSocket,
    mbuf: &mut MsgBuf,
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_out(timeout).await {
            Ok(()) => loop {
                match soc.as_socket().send_msg(mbuf) {
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

pub(crate) async fn async_read_some(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_in(timeout).await {
            Ok(()) => loop {
                match soc.as_socket().read(buf) {
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

pub(crate) async fn async_receive(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_in(timeout).await {
            Ok(()) => loop {
                match soc.as_socket().receive(buf) {
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

pub(crate) async fn async_receive_from<E>(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<(usize, E)>
where
    E: Endpoint,
{
    loop {
        match soc.poll_in(timeout).await {
            Ok(()) => loop {
                match soc.as_socket().receive_from(buf) {
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

pub(crate) async fn async_receive_msg(
    soc: &AsyncSocket,
    mbuf: &mut MsgBuf,
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_in(timeout).await {
            Ok(()) => loop {
                match soc.as_socket().receive_msg(mbuf) {
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
