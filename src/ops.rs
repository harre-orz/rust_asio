use crate::signal_set::Signal;
use crate::{ffi, Endpoint, IoContext, OsError, Protocol};
use std::os::fd::OwnedFd;
use std::time::Duration;

pub fn accept<E>(ctx: &IoContext, soc: &OwnedFd, timeout: Duration) -> Result<(OwnedFd, E), OsError>
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
    ctx: &IoContext,
    soc: &OwnedFd,
    timeout: Duration,
) -> Result<(OwnedFd, E), OsError>
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

pub fn connect<P>(pro: P, ep: &P::Endpoint, timeout: Duration) -> Result<OwnedFd, OsError>
where
    P: Protocol,
{
    let soc = ffi::socket(pro)?;
    let soc = loop {
        match ffi::connect(&soc, ep) {
            Ok(_) => break soc,
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = ffi::wait_for_readable(&soc, timeout) {
                    return Err(err);
                } else {
                    break soc;
                }
            }
            Err(OsError::INTERRUPTED) => {}
            Err(err) => return Err(err),
        }
    };
    Ok(soc)
}

pub async fn async_connect<P>(
    pro: P,
    ep: &P::Endpoint,
    timeout: Duration,
) -> Result<OwnedFd, OsError>
where
    P: Protocol,
{
    let soc = ffi::socket(pro)?;
    let soc = loop {
        match ffi::connect(&soc, ep) {
            Ok(_) => break soc,
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = ffi::wait_for_readable(&soc, timeout) {
                    return Err(err);
                } else {
                    break soc;
                }
            }
            Err(OsError::INTERRUPTED) => {}
            Err(err) => return Err(err),
        }
    };
    Ok(soc)
}

pub fn write_some(
    ctx: &IoContext,
    soc: &OwnedFd,
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
    ctx: &IoContext,
    soc: &OwnedFd,
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

pub fn send(
    ctx: &IoContext,
    soc: &OwnedFd,
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
    ctx: &IoContext,
    soc: &OwnedFd,
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

pub fn send_to<E>(
    ctx: &IoContext,
    soc: &OwnedFd,
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
    ctx: &IoContext,
    soc: &OwnedFd,
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

pub fn read_some(
    ctx: &IoContext,
    soc: &OwnedFd,
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
    ctx: &IoContext,
    soc: &OwnedFd,
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

pub fn receive(
    ctx: &IoContext,
    soc: &OwnedFd,
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
    ctx: &IoContext,
    soc: &OwnedFd,
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

pub fn receive_from<E>(
    ctx: &IoContext,
    soc: &OwnedFd,
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
    ctx: &IoContext,
    soc: &OwnedFd,
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

pub fn signal_read(ctx: &IoContext, soc: &OwnedFd, timeout: Duration) -> Result<Signal, OsError> {
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

pub async fn async_signal_read(
    ctx: &IoContext,
    soc: &OwnedFd,
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
