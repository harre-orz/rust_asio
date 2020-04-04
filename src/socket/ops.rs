//

#[cfg(unix)]
use super::{accept, connect, read, recv, recvfrom, send, sendto, write};
use error::{
    ErrorCode, INTERRUPTED, IN_PROGRESS, OPERATION_CANCELED, SUCCESS, TRY_AGAIN, WOULD_BLOCK,
};
use executor::{IoContext, Wait};
use socket_base::{Protocol, Socket};

pub fn nb_accept<P, S>(soc: &S, pro: P, ctx: &IoContext) -> Result<(P::Socket, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    accept(soc, pro, ctx)
}

pub fn nb_connect<P, S>(soc: &S, ep: &P::Endpoint) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    connect(soc, ep)
}


pub fn nb_read_some<P, S>(soc: &S, buf: &mut [u8]) -> Result<usize, ErrorCode>
where
    S: Socket<P>,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    read(soc, buf)
}


pub fn nb_receive<P, S>(soc: &S, buf: &mut [u8], flags: i32) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.is_stopped() {
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
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    recvfrom(soc, buf, flags, pro)
}


pub fn nb_send<P, S>(soc: &S, buf: &[u8], flags: i32) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.is_stopped() {
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
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    sendto(soc, buf, flags, ep)
}

pub fn nb_write_some<P, S>(soc: &S, buf: &[u8]) -> Result<usize, ErrorCode>
where
    S: Socket<P>,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    write(soc, buf)
}

pub fn wa_accept<P, S, W>(
    soc: &S,
    pro: &P,
    ctx: &IoContext,
    wait: &mut W,
) -> Result<(P::Socket, P::Endpoint), ErrorCode>
where
    P: Protocol + Clone,
    S: Socket<P>,
    W: Wait,
{
    loop {
        if soc.is_stopped() {
            return Err(OPERATION_CANCELED);
        }
        match accept(soc, pro.clone(), ctx) {
            Ok(soc) => return Ok(soc),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = wait.readable(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            err => return err,
        }
    }
}

pub fn wa_connect<P, S, W>(soc: &S, ep: &P::Endpoint, wait: &mut W) -> Result<(), ErrorCode>
    where
    P: Protocol,
    S: Socket<P>,
    W: Wait,
{
    loop {
        if soc.is_stopped() {
            return Err(OPERATION_CANCELED);
        }
        match connect(soc, ep) {
            Ok(_) => return Ok(()),
            Err(IN_PROGRESS) | Err(WOULD_BLOCK) => {
                let err = wait.writable(soc);
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

pub fn wa_read_some<P, S, W>(soc: &S, buf: &mut [u8], wait: &mut W) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    W: Wait,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match read(soc, buf) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = wait.readable(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn wa_receive<P, S, W>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
    wait: &mut W,
) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    W: Wait,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match recv(soc, buf, flags) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = wait.readable(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn wa_receive_from<P, S, W>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
    pro: &P,
    wait: &mut W,
) -> Result<(usize, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    W: Wait,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match recvfrom(soc, buf, flags, pro) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = wait.readable(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn wa_send<P, S, W>(soc: &S, buf: &[u8], flags: i32, wait: &mut W) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    W: Wait,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match send(soc, buf, flags) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = wait.writable(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn wa_send_to<P, S, W>(
    soc: &S,
    buf: &[u8],
    flags: i32,
    ep: &P::Endpoint,
    wait: &mut W,
) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    W: Wait,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match sendto(soc, buf, flags, ep) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = wait.writable(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub fn wa_write_some<P, S, W>(soc: &S, buf: &[u8], wait: &mut W) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
    W: Wait,
{
    if soc.is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match write(soc, buf) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = wait.writable(soc);
                if err != SUCCESS {
                    return Err(err);
                }
            }
            Err(INTERRUPTED) => {
                if soc.is_stopped() {
                    return Err(OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}
