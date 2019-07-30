//

use super::{accept, connect, read, recv, recvfrom, send, sendto, write};
use error::{
    ErrorCode, INTERRUPTED, IN_PROGRESS, OPERATION_CANCELED, SUCCESS, TRY_AGAIN, WOULD_BLOCK,
};
use executor::Ready;
use socket_base::{Protocol, Socket};

pub fn bk_accept<P, S, R>(
    soc: &S,
    pro: &P,
    ready: &mut R,
) -> Result<(P::Socket, P::Endpoint), ErrorCode>
where
    P: Protocol + Clone,
    S: Socket<P>,
    R: Ready,
{
    loop {
        if soc.as_ctx().is_stopped() {
            return Err(OPERATION_CANCELED);
        }
        match accept(soc, pro.clone()) {
            Ok(soc) => return Ok(soc),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = ready.ready_reading(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            err => return err,
        }
    }
}

pub fn bk_connect<P, S, R>(soc: &S, ep: &P::Endpoint, ready: &mut R) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    R: Ready,
{
    loop {
        if soc.as_ctx().is_stopped() {
            return Err(OPERATION_CANCELED);
        }
        match connect(soc, ep) {
            Ok(_) => return Ok(()),
            Err(IN_PROGRESS) | Err(WOULD_BLOCK) => {
                let err = ready.ready_writing(soc);
                if err == SUCCESS {
                    return Ok(());
                } else {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => (),
            err => return err,
        }
    }
}

pub fn bk_read_some<P, S, R>(soc: &S, buf: &mut [u8], ready: &mut R) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    R: Ready,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match read(soc, buf) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = ready.ready_reading(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn bk_receive<P, S, R>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
    ready: &mut R,
) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    R: Ready,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match recv(soc, buf, flags) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = ready.ready_reading(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn bk_receive_from<P, S, R>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
    pro: &P,
    ready: &mut R,
) -> Result<(usize, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    R: Ready,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match recvfrom(soc, buf, flags, pro) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = ready.ready_reading(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn bk_send<P, S, R>(soc: &S, buf: &[u8], flags: i32, ready: &mut R) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    R: Ready,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match send(soc, buf, flags) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = ready.ready_writing(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn bk_send_to<P, S, R>(
    soc: &S,
    buf: &[u8],
    flags: i32,
    ep: &P::Endpoint,
    ready: &mut R,
) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    R: Ready,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match sendto(soc, buf, flags, ep) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = ready.ready_writing(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn bk_write_some<P, S, R>(soc: &S, buf: &[u8], ready: &mut R) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    R: Ready,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match write(soc, buf) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = ready.ready_writing(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.as_ctx().is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn nb_accept<P, S>(soc: &S, pro: P) -> Result<(P::Socket, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    accept(soc, pro)
}

pub fn nb_connect<P, S>(soc: &S, ep: &P::Endpoint) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    connect(soc, ep)
}

pub fn nb_receive<P, S>(soc: &S, buf: &mut [u8], flags: i32) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    recv(soc, buf, flags)
}

pub fn nb_receive_from<P, S>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
    pro: &P,
) -> Result<(usize, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    recvfrom(soc, buf, flags, pro)
}

pub fn nb_read_some<P, S>(soc: &S, buf: &mut [u8]) -> Result<usize, ErrorCode>
where
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    read(soc, buf)
}

pub fn nb_send<P, S>(soc: &S, buf: &[u8], flags: i32) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    send(soc, buf, flags)
}

pub fn nb_send_to<P, S>(
    soc: &S,
    buf: &[u8],
    flags: i32,
    ep: &P::Endpoint,
) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    sendto(soc, buf, flags, ep)
}

pub fn nb_write_some<P, S>(soc: &S, buf: &[u8]) -> Result<usize, ErrorCode>
where
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    write(soc, buf)
}
