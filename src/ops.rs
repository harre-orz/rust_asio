use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, Signal, Socket};
use crate::socket_base::Endpoint;
use std::time::Instant;

type Result<T> = std::result::Result<T, OsError>;

pub fn connect<E>(soc: &Socket, ep: &E, ctx: &IoContext, time: Option<Instant>) -> Result<()>
where
    E: Endpoint,
{
    loop {
        match ffi::connect(soc, ep) {
            Ok(_) => break,
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = ffi::wait_for_writable(&soc, time) {
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

pub async fn async_connect<E>(
    soc: &AsyncSocket,
    ep: &E,
) -> Result<()>
where
    E: Endpoint,
{
    loop {
        match ffi::connect(soc.as_socket(), ep) {
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

pub fn accept<E>(soc: &Socket, ctx: &IoContext, exp: Option<Instant>) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    loop {
        match ffi::wait_for_readable(soc, exp) {
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

pub async fn async_accept<E>(soc: &AsyncSocket) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable().await {
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
    soc: &Socket,
    buf: &[u8],
    ctx: &IoContext,
    time: Option<Instant>
) -> Result<usize> {
    loop {
        match ffi::wait_for_writable(soc, time) {
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
) -> Result<usize> {
    loop {
        match soc.wait_for_writable().await {
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

pub fn send(soc: &Socket, buf: &[u8], ctx: &IoContext, time: Option<Instant>) -> Result<usize> {
    loop {
        match ffi::wait_for_writable(soc, time) {
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

pub async fn async_send(soc: &AsyncSocket, buf: &[u8]) -> Result<usize> {
    loop {
        match soc.wait_for_writable().await {
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
    soc: &Socket,
    buf: &[u8],
    ep: &E,
    ctx: &IoContext,
    time: Option<Instant>,
) -> Result<usize>
where
    E: Endpoint,
{
    loop {
        match ffi::wait_for_writable(soc, time) {
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
) -> Result<usize>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_writable().await {
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
    soc: &Socket,
    buf: &mut [u8],
    ctx: &IoContext,
    time: Option<Instant>,
) -> Result<usize> {
    loop {
        match ffi::wait_for_readable(soc, time) {
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
) -> Result<usize> {
    loop {
        match soc.wait_for_readable().await {
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
    soc: &Socket,
    buf: &mut [u8],
    ctx: &IoContext,
    time: Option<Instant>,
) -> Result<usize> {
    loop {
        match ffi::wait_for_readable(soc, time) {
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
) -> Result<usize> {
    loop {
        match soc.wait_for_readable().await {
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
    soc: &Socket,
    buf: &mut [u8],
    ctx: &IoContext,
    time: Option<Instant>,
) -> Result<(usize, E)>
where
    E: Endpoint,
{
    loop {
        match ffi::wait_for_readable(soc, time) {
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
) -> Result<(usize, E)>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable().await {
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

pub fn signal_read(soc: &Socket, ctx: &IoContext, time: Option<Instant>) -> Result<Signal> {
    loop {
        match ffi::wait_for_readable(soc, time) {
            Ok(()) => loop {
                match ffi::signal_read(soc) {
                    Ok(sig) => return Ok(sig),
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

pub async fn async_signal_read(soc: &AsyncSocket) -> Result<Signal> {
    loop {
        match soc.wait_for_readable().await {
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
