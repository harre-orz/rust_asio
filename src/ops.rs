use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket, Signal, Timeout};
use crate::socket_base::Endpoint;

pub fn connect<E>(
    soc: ConnectedSocket,
    ep: &E,
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<ConnectedSocket, OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::connect(&soc, ep) {
            Ok(_) => break,
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = ffi::wait_for_writable(&soc, timeout) {
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
    Ok(soc)
}

pub async fn async_connect<E>(
    soc: ConnectedSocket,
    ep: &E,
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<AsyncSocket, OsError>
where
    E: Endpoint,
{
    let soc = AsyncSocket::new(ctx.clone(), soc);
    loop {
        match ffi::connect(soc.as_socket(), ep) {
            Ok(_) => break,
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_writable(timeout).await {
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
    Ok(soc)
}

pub fn accept<E>(
    soc: &ConnectedSocket,
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<(ConnectedSocket, E), OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::wait_for_readable(soc, timeout) {
            Ok(()) => loop {
                match ffi::accept(soc) {
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

pub async fn async_accept<E>(
    soc: &AsyncSocket,
    timeout: Timeout,
) -> Result<(ConnectedSocket, E), OsError>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable(timeout).await {
            Ok(()) => loop {
                match ffi::accept(soc.as_socket()) {
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
    soc: &ConnectedSocket,
    buf: &[u8],
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<usize, OsError> {
    loop {
        match ffi::wait_for_writable(soc, timeout) {
            Ok(()) => loop {
                match ffi::write(soc, buf) {
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

pub async fn async_write_some(
    soc: &AsyncSocket,
    buf: &[u8],
    timeout: Timeout,
) -> Result<usize, OsError> {
    loop {
        match soc.wait_for_writable(timeout).await {
            Ok(()) => loop {
                match ffi::write(soc.as_socket(), buf) {
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

pub fn send(
    soc: &ConnectedSocket,
    buf: &[u8],
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<usize, OsError> {
    loop {
        match ffi::wait_for_writable(soc, timeout) {
            Ok(()) => loop {
                match ffi::send(soc, buf) {
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

pub async fn async_send(soc: &AsyncSocket, buf: &[u8], timeout: Timeout) -> Result<usize, OsError> {
    loop {
        match soc.wait_for_writable(timeout).await {
            Ok(()) => loop {
                match ffi::send(soc.as_socket(), buf) {
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
    soc: &ConnectedSocket,
    buf: &[u8],
    ep: &E,
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<usize, OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::wait_for_writable(soc, timeout) {
            Ok(()) => loop {
                match ffi::send_to(soc, buf, ep) {
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

pub async fn async_send_to<E>(
    soc: &AsyncSocket,
    buf: &[u8],
    ep: &E,
    timeout: Timeout,
) -> Result<usize, OsError>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_writable(timeout).await {
            Ok(()) => loop {
                match ffi::send_to(soc.as_socket(), buf, ep) {
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
    soc: &ConnectedSocket,
    buf: &mut [u8],
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<usize, OsError> {
    loop {
        match ffi::wait_for_readable(soc, timeout) {
            Ok(()) => loop {
                match ffi::read(soc, buf) {
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

pub async fn async_read_some(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<usize, OsError> {
    loop {
        match soc.wait_for_readable(timeout).await {
            Ok(()) => loop {
                match ffi::read(soc.as_socket(), buf) {
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
    soc: &ConnectedSocket,
    buf: &mut [u8],
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<usize, OsError> {
    loop {
        match ffi::wait_for_readable(soc, timeout) {
            Ok(()) => loop {
                match ffi::receive(soc, buf) {
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

pub async fn async_receive(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<usize, OsError> {
    loop {
        match soc.wait_for_readable(timeout).await {
            Ok(()) => loop {
                match ffi::receive(soc.as_socket(), buf) {
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
    soc: &ConnectedSocket,
    buf: &mut [u8],
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<(usize, E), OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::wait_for_readable(soc, timeout) {
            Ok(()) => loop {
                match ffi::receive_from(soc, buf) {
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

pub async fn async_receive_from<E>(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<(usize, E), OsError>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable(timeout).await {
            Ok(()) => loop {
                match ffi::receive_from(soc.as_socket(), buf) {
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

pub fn signal_read(
    soc: &ConnectedSocket,
    timeout: Timeout,
    ctx: &IoContext,
) -> Result<Signal, OsError> {
    loop {
        match ffi::wait_for_readable(soc, timeout) {
            Ok(()) => loop {
                match ffi::signal_read(soc) {
                    Ok(sig) => return Ok(sig),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                        if let Err(err) = ffi::wait_for_readable(soc, timeout) {
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

pub async fn async_signal_read(soc: &AsyncSocket, timeout: Timeout) -> Result<Signal, OsError> {
    loop {
        match soc.wait_for_readable(timeout).await {
            Ok(()) => loop {
                match ffi::signal_read(soc.as_socket()) {
                    Ok(sig) => return Ok(sig),
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
