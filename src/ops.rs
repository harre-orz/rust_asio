use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket, Signal};
use crate::socket_base::{Endpoint, Protocol};
use std::time::Duration;

pub fn accept<E>(
    ctx: &IoContext,
    soc: &ConnectedSocket,
    timeout: Duration,
) -> Result<(ConnectedSocket, E), OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::accept(soc) {
            Ok(soc) => return Ok(soc),
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
    }
}

pub async fn async_accept<E>(
    soc: &AsyncSocket,
    timeout: Duration,
) -> Result<(ConnectedSocket, E), OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::accept(soc.as_socket()) {
            Ok(soc) => return Ok(soc),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_readable(timeout).await {
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

pub fn connect<P>(
    ctx: &IoContext,
    pro: P,
    ep: &P::Endpoint,
    timeout: Duration,
) -> Result<ConnectedSocket, OsError>
where
    P: Protocol,
{
    let soc = ffi::socket(pro)?;
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

pub async fn async_connect<P>(
    ctx: &IoContext,
    pro: P,
    ep: &P::Endpoint,
    timeout: Duration,
) -> Result<AsyncSocket, OsError>
where
    P: Protocol,
{
    let soc = ffi::socket(pro)?;
    let soc = ctx.async_socket(soc);
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

pub fn write_some(
    ctx: &IoContext,
    soc: &ConnectedSocket,
    buf: &[u8],
    timeout: Duration,
) -> Result<usize, OsError> {
    loop {
        match ffi::write(soc, buf) {
            Ok(len) => return Ok(len),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = ffi::wait_for_writable(soc, timeout) {
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

pub async fn async_write_some(
    soc: &AsyncSocket,
    buf: &[u8],
    timeout: Duration,
) -> Result<usize, OsError> {
    loop {
        match ffi::write(soc.as_socket(), buf) {
            Ok(len) => return Ok(len),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_writable(timeout).await {
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

pub fn send(
    ctx: &IoContext,
    soc: &ConnectedSocket,
    buf: &[u8],
    timeout: Duration,
) -> Result<usize, OsError> {
    loop {
        match ffi::send(soc, buf) {
            Ok(len) => return Ok(len),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = ffi::wait_for_writable(soc, timeout) {
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

pub async fn async_send(
    soc: &AsyncSocket,
    buf: &[u8],
    timeout: Duration,
) -> Result<usize, OsError> {
    loop {
        match ffi::send(soc.as_socket(), buf) {
            Ok(len) => return Ok(len),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_writable(timeout).await {
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

pub fn send_to<E>(
    ctx: &IoContext,
    soc: &ConnectedSocket,
    buf: &[u8],
    ep: &E,
    timeout: Duration,
) -> Result<usize, OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::send_to(soc, buf, ep) {
            Ok(len) => return Ok(len),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = ffi::wait_for_writable(soc, timeout) {
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

pub async fn async_send_to<E>(
    soc: &AsyncSocket,
    buf: &[u8],
    ep: &E,
    timeout: Duration,
) -> Result<usize, OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::send_to(soc.as_socket(), buf, ep) {
            Ok(len) => return Ok(len),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_writable(timeout).await {
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

pub fn read_some(
    ctx: &IoContext,
    soc: &ConnectedSocket,
    buf: &mut [u8],
    timeout: Duration,
) -> Result<usize, OsError> {
    loop {
        match ffi::read(soc, buf) {
            Ok(len) => return Ok(len),
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
    }
}

pub async fn async_read_some(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Duration,
) -> Result<usize, OsError> {
    loop {
        match ffi::read(soc.as_socket(), buf) {
            Ok(len) => return Ok(len),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_readable(timeout).await {
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

pub fn receive(
    ctx: &IoContext,
    soc: &ConnectedSocket,
    buf: &mut [u8],
    timeout: Duration,
) -> Result<usize, OsError> {
    loop {
        match ffi::receive(soc, buf) {
            Ok(len) => return Ok(len),
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
    }
}

pub async fn async_receive(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Duration,
) -> Result<usize, OsError> {
    loop {
        match ffi::receive(soc.as_socket(), buf) {
            Ok(len) => return Ok(len),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_readable(timeout).await {
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

pub fn receive_from<E>(
    ctx: &IoContext,
    soc: &ConnectedSocket,
    buf: &mut [u8],
    timeout: Duration,
) -> Result<(usize, E), OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::receive_from(soc, buf) {
            Ok(len) => return Ok(len),
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
    }
}

pub async fn async_receive_from<E>(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Duration,
) -> Result<(usize, E), OsError>
where
    E: Endpoint,
{
    loop {
        match ffi::receive_from(soc.as_socket(), buf) {
            Ok(len) => return Ok(len),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_readable(timeout).await {
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

pub fn signal_read(
    ctx: &IoContext,
    soc: &ConnectedSocket,
    timeout: Duration,
) -> Result<Signal, OsError> {
    loop {
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
    }
}

pub async fn async_signal_read(soc: &AsyncSocket, timeout: Duration) -> Result<Signal, OsError> {
    loop {
        match ffi::signal_read(soc.as_socket()) {
            Ok(sig) => return Ok(sig),
            #[allow(unreachable_patterns)]
            Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_readable(timeout).await {
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
