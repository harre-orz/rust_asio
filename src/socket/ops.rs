use super::{accept, connect, read, readable, recv, recvfrom, send, sendto, writable, write, Timeout};
use error::{ErrorCode, INTERRUPTED, IN_PROGRESS, OPERATION_CANCELED, SUCCESS, TRY_AGAIN, WOULD_BLOCK};
use executor::{SocketContext, YieldContext};
use socket_base::{Protocol, Socket};

pub trait AsSocketContext {
    fn as_socket_ctx(&mut self) -> &mut SocketContext;
}

pub fn async_accept<P, S>(
    soc: &mut S,
    pro: &P,
    yield_ctx: &mut YieldContext,
) -> Result<(P::Socket, P::Endpoint), ErrorCode>
where
    P: Protocol + Clone,
    S: Socket<P> + AsSocketContext,
{
    loop {
        if soc.as_ctx().is_stopped() {
            return Err(OPERATION_CANCELED);
        }
        match accept(soc, pro.clone()) {
            Ok(soc) => return Ok(soc),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = yield_ctx.yield_readable(soc.as_socket_ctx());
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

pub fn bk_accept<P, S>(soc: &S, pro: &P, timeout: Timeout) -> Result<(P::Socket, P::Endpoint), ErrorCode>
where
    P: Protocol + Clone,
    S: Socket<P>,
{
    loop {
        if soc.as_ctx().is_stopped() {
            return Err(OPERATION_CANCELED);
        }
        match accept(soc, pro.clone()) {
            Ok(soc) => return Ok(soc),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = readable(soc, timeout);
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

pub fn async_connect<P, S>(soc: &mut S, ep: &P::Endpoint, yield_ctx: &mut YieldContext) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P> + AsSocketContext,
{
    loop {
        if soc.as_ctx().is_stopped() {
            return Err(OPERATION_CANCELED);
        }
        match connect(soc, ep) {
            Ok(_) => return Ok(()),
            Err(IN_PROGRESS) | Err(WOULD_BLOCK) => {
                let err = yield_ctx.yield_writable(soc.as_socket_ctx());
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

pub fn bk_connect<P, S>(soc: &S, ep: &P::Endpoint, timeout: Timeout) -> Result<(), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    loop {
        if soc.as_ctx().is_stopped() {
            return Err(OPERATION_CANCELED);
        }
        match connect(soc, ep) {
            Ok(_) => return Ok(()),
            Err(IN_PROGRESS) | Err(WOULD_BLOCK) => {
                let err = writable(soc, timeout);
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

pub fn async_read_some<P, S>(soc: &mut S, buf: &mut [u8], yield_ctx: &mut YieldContext) -> Result<usize, ErrorCode>
where
    S: Socket<P> + AsSocketContext,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match read(soc, buf) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = yield_ctx.yield_readable(soc.as_socket_ctx());
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

pub fn async_receive<P, S>(
    soc: &mut S,
    buf: &mut [u8],
    flags: i32,
    yield_ctx: &mut YieldContext,
) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P> + AsSocketContext,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match recv(soc, buf, flags) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = yield_ctx.yield_readable(soc.as_socket_ctx());
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

pub fn async_receive_from<P, S>(
    soc: &mut S,
    buf: &mut [u8],
    flags: i32,
    pro: &P,
    yield_ctx: &mut YieldContext,
) -> Result<(usize, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P> + AsSocketContext,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match recvfrom(soc, buf, flags, pro) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = yield_ctx.yield_readable(soc.as_socket_ctx());
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

pub fn bk_read_some<P, S>(soc: &S, buf: &mut [u8], timeout: Timeout) -> Result<usize, ErrorCode>
where
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match read(soc, buf) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = readable(soc, timeout);
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

pub fn bk_receive<P, S>(soc: &S, buf: &mut [u8], flags: i32, timeout: Timeout) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match recv(soc, buf, flags) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = readable(soc, timeout);
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

pub fn bk_receive_from<P, S>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
    pro: &P,
    timeout: Timeout,
) -> Result<(usize, P::Endpoint), ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match recvfrom(soc, buf, flags, pro) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = readable(soc, timeout);
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

pub fn nb_receive_from<P, S>(soc: &S, buf: &mut [u8], flags: i32, pro: &P) -> Result<(usize, P::Endpoint), ErrorCode>
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

pub fn async_send<P, S>(soc: &mut S, buf: &[u8], flags: i32, yield_ctx: &mut YieldContext) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P> + AsSocketContext,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match send(soc, buf, flags) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = yield_ctx.yield_writable(soc.as_socket_ctx());
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

pub fn async_send_to<P, S>(
    soc: &mut S,
    buf: &[u8],
    flags: i32,
    ep: &P::Endpoint,
    yield_ctx: &mut YieldContext,
) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P> + AsSocketContext,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match sendto(soc, buf, flags, ep) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = yield_ctx.yield_writable(soc.as_socket_ctx());
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

pub fn async_write_some<P, S>(soc: &mut S, buf: &[u8], yield_ctx: &mut YieldContext) -> Result<usize, ErrorCode>
where
    S: Socket<P> + AsSocketContext,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match write(soc, buf) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = yield_ctx.yield_writable(soc.as_socket_ctx());
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

pub fn bk_send<P, S>(soc: &S, buf: &[u8], flags: i32, timeout: Timeout) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match send(soc, buf, flags) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = writable(soc, timeout);
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

pub fn bk_send_to<P, S>(soc: &S, buf: &[u8], flags: i32, ep: &P::Endpoint, timeout: Timeout) -> Result<usize, ErrorCode>
where
    P: Protocol,
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match sendto(soc, buf, flags, ep) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = writable(soc, timeout);
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

pub fn bk_write_some<P, S>(soc: &S, buf: &[u8], timeout: Timeout) -> Result<usize, ErrorCode>
where
    S: Socket<P>,
{
    if soc.as_ctx().is_stopped() {
        return Err(OPERATION_CANCELED);
    }
    loop {
        match write(soc, buf) {
            Ok(size) => return Ok(size),
            #[allow(unreachable_patterns)]
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                let err = writable(soc, timeout);
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

pub fn nb_send_to<P, S>(soc: &S, buf: &[u8], flags: i32, ep: &P::Endpoint) -> Result<usize, ErrorCode>
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
