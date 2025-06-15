use crate::IoContext;
use crate::error::OsError;
use crate::exec::async_socket::AsyncSocket;
use crate::ffi::socket::Socket;
use crate::socket_base::Endpoint;
use std::result;
use std::time::Instant;

type Result<T> = result::Result<T, OsError>;

pub fn connect<E>(soc: &Socket, ep: &E, ctx: &IoContext, time: Option<Instant>) -> Result<()>
where
    E: Endpoint,
{
    loop {
        match soc.nb_connect(ep) {
            Ok(_) => break,
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_writable(time) {
                    return Err(err);
                } else {
                    break;
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
    Ok(())
}

pub async fn async_connect<E>(soc: &AsyncSocket, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    loop {
        match soc.as_socket().nb_connect(ep) {
            Ok(_) => break,
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_writable().await {
                    return Err(err);
                } else {
                    break;
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
    Ok(())
}

pub fn accept<E>(soc: &Socket, ctx: &IoContext, cto: Option<Instant>) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable(cto) {
            Ok(()) => loop {
                match soc.nb_accept() {
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

pub async fn async_accept<E>(soc: &AsyncSocket) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable().await {
            Ok(()) => loop {
                match soc.as_socket().nb_accept() {
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

pub fn write_some(
    soc: &Socket,
    buf: &[u8],
    ctx: &IoContext,
    cto: Option<Instant>,
) -> Result<usize> {
    loop {
        match soc.wait_for_writable(cto) {
            Ok(()) => loop {
                match soc.nb_write(buf) {
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

pub async fn async_write_some(soc: &AsyncSocket, buf: &[u8]) -> Result<usize> {
    loop {
        match soc.wait_for_writable().await {
            Ok(()) => loop {
                match soc.as_socket().nb_write(buf) {
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

pub fn send(soc: &Socket, buf: &[u8], ctx: &IoContext, cto: Option<Instant>) -> Result<usize> {
    loop {
        match soc.wait_for_writable(cto) {
            Ok(()) => loop {
                match soc.nb_send(buf) {
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

pub async fn async_send(soc: &AsyncSocket, buf: &[u8]) -> Result<usize> {
    loop {
        match soc.wait_for_writable().await {
            Ok(()) => loop {
                match soc.as_socket().nb_send(buf) {
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

pub fn send_to<E>(
    soc: &Socket,
    buf: &[u8],
    ep: &E,
    ctx: &IoContext,
    cto: Option<Instant>,
) -> Result<usize>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_writable(cto) {
            Ok(()) => loop {
                match soc.nb_send_to(buf, ep) {
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

pub async fn async_send_to<E>(soc: &AsyncSocket, buf: &[u8], ep: &E) -> Result<usize>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_writable().await {
            Ok(()) => loop {
                match soc.as_socket().nb_send_to(buf, ep) {
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

pub fn read_some(
    soc: &Socket,
    buf: &mut [u8],
    ctx: &IoContext,
    time: Option<Instant>,
) -> Result<usize> {
    loop {
        match soc.wait_for_readable(time) {
            Ok(()) => loop {
                match soc.nb_read(buf) {
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

pub async fn async_read_some(soc: &AsyncSocket, buf: &mut [u8]) -> Result<usize> {
    loop {
        match soc.wait_for_readable().await {
            Ok(()) => loop {
                match soc.as_socket().nb_read(buf) {
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

pub fn receive(
    soc: &Socket,
    buf: &mut [u8],
    ctx: &IoContext,
    time: Option<Instant>,
) -> Result<usize> {
    loop {
        match soc.wait_for_readable(time) {
            Ok(()) => loop {
                match soc.nb_receive(buf) {
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

pub async fn async_receive(soc: &AsyncSocket, buf: &mut [u8]) -> Result<usize> {
    loop {
        match soc.wait_for_readable().await {
            Ok(()) => loop {
                match soc.as_socket().nb_receive(buf) {
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

pub fn receive_from<E>(
    soc: &Socket,
    buf: &mut [u8],
    ctx: &IoContext,
    time: Option<Instant>,
) -> Result<(usize, E)>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable(time) {
            Ok(()) => loop {
                match soc.nb_receive_from(buf) {
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

pub async fn async_receive_from<E>(soc: &AsyncSocket, buf: &mut [u8]) -> Result<(usize, E)>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable().await {
            Ok(()) => loop {
                match soc.as_socket().nb_receive_from(buf) {
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
