use crate::IoContext;
use crate::buffer::MsgBuf;
use crate::error::OsError;
use crate::exec::AsyncSocket;
use crate::socket::Socket;
use crate::socket_base::{Endpoint, EndpointRef};
use std::cell::Cell;
use std::result;
use std::time::{Duration, Instant};

type Result<T> = result::Result<T, OsError>;

pub(crate) struct Blocking {
    ctx: IoContext,
    timeout: Cell<Option<Instant>>,
}

impl Blocking {
    pub(crate) const fn new(ctx: IoContext) -> Self {
        Self {
            ctx: ctx,
            timeout: Cell::new(None),
        }
    }

    pub(crate) const fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub(crate) fn into_ctx(self) -> IoContext {
        self.ctx
    }

    pub(crate) fn expires_at(&self, timeout: Instant) {
        self.timeout.set(Some(timeout))
    }

    pub(crate) fn expires_from_now(&self, timeout: Duration) {
        self.timeout.set(Some(Instant::now() + timeout))
    }

    fn timeout(time: Option<Instant>) -> i32 {
        if let Some(time) = time {
            let time = time.duration_since(Instant::now()).as_millis();
            if time > i32::MAX as u128 {
                -1
            } else {
                time as i32
            }
        } else {
            -1
        }
    }

    fn wait_for_readable(&self, soc: &Socket) -> Result<()> {
        soc.poll_in(Self::timeout(self.timeout.take()))
    }

    fn wait_for_writable(&self, soc: &Socket) -> Result<()> {
        soc.poll_out(Self::timeout(self.timeout.take()))
    }
}

pub(crate) fn connect<E>(soc: &Socket, ep: &EndpointRef<E>, blk: &Blocking) -> Result<()>
where
    E: Endpoint,
{
    loop {
        match soc.connect(ep) {
            Ok(_) => return Ok(()),
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = blk.wait_for_writable(soc) {
                    return Err(err);
                }
            }
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) async fn async_connect<E>(soc: &AsyncSocket, ep: &EndpointRef<'_, E>) -> Result<()>
where
    E: Endpoint,
{
    loop {
        match soc.as_socket().connect(ep) {
            Ok(_) => return Ok(()),
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.wait_for_writable().await {
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

pub(crate) fn accept<E>(soc: &Socket, blk: &Blocking) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    loop {
        match blk.wait_for_readable(soc) {
            Ok(()) => loop {
                match soc.accept() {
                    Ok(soc) => return Ok(soc),
                    Err(OsError::INTERRUPTED) => {
                        if blk.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) async fn async_accept<E>(soc: &AsyncSocket) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable().await {
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

pub(crate) fn write_some(soc: &Socket, buf: &[u8], blk: &Blocking) -> Result<usize> {
    loop {
        match blk.wait_for_writable(soc) {
            Ok(()) => loop {
                match soc.write(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if blk.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) async fn async_write_some(soc: &AsyncSocket, buf: &[u8]) -> Result<usize> {
    loop {
        match soc.wait_for_writable().await {
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

pub(crate) fn send(soc: &Socket, buf: &[u8], blk: &Blocking) -> Result<usize> {
    loop {
        match blk.wait_for_writable(soc) {
            Ok(()) => loop {
                match soc.send(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if blk.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) async fn async_send(soc: &AsyncSocket, buf: &[u8]) -> Result<usize> {
    loop {
        match soc.wait_for_writable().await {
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

pub(crate) fn send_to<E>(
    soc: &Socket,
    buf: &[u8],
    ep: &EndpointRef<E>,
    blk: &Blocking,
) -> Result<usize>
where
    E: Endpoint,
{
    loop {
        match blk.wait_for_writable(soc) {
            Ok(()) => loop {
                match soc.send_to(buf, ep) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if blk.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
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
) -> Result<usize>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_writable().await {
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

pub(crate) fn send_msg(soc: &Socket, mbuf: &mut MsgBuf, blk: &Blocking) -> Result<usize> {
    loop {
        match blk.wait_for_writable(soc) {
            Ok(()) => loop {
                match soc.send_msg(mbuf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if blk.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) async fn async_send_msg(soc: &AsyncSocket, mbuf: &mut MsgBuf) -> Result<usize> {
    loop {
        match soc.wait_for_writable().await {
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

pub(crate) fn read_some(soc: &Socket, buf: &mut [u8], blk: &Blocking) -> Result<usize> {
    loop {
        match blk.wait_for_readable(soc) {
            Ok(()) => loop {
                match soc.read(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if blk.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) async fn async_read_some(soc: &AsyncSocket, buf: &mut [u8]) -> Result<usize> {
    loop {
        match soc.wait_for_readable().await {
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

pub(crate) fn receive(soc: &Socket, buf: &mut [u8], blk: &Blocking) -> Result<usize> {
    loop {
        match blk.wait_for_readable(soc) {
            Ok(()) => loop {
                match soc.receive(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if blk.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) async fn async_receive(soc: &AsyncSocket, buf: &mut [u8]) -> Result<usize> {
    loop {
        match soc.wait_for_readable().await {
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

pub(crate) fn receive_from<E>(soc: &Socket, buf: &mut [u8], blk: &Blocking) -> Result<(usize, E)>
where
    E: Endpoint,
{
    loop {
        match blk.wait_for_readable(soc) {
            Ok(()) => loop {
                match soc.receive_from(buf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if blk.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) async fn async_receive_from<E>(soc: &AsyncSocket, buf: &mut [u8]) -> Result<(usize, E)>
where
    E: Endpoint,
{
    loop {
        match soc.wait_for_readable().await {
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

pub(crate) fn receive_msg(soc: &Socket, mbuf: &mut MsgBuf, blk: &Blocking) -> Result<usize> {
    loop {
        match blk.wait_for_readable(soc) {
            Ok(()) => loop {
                match soc.receive_msg(mbuf) {
                    Ok(len) => return Ok(len),
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                    Err(OsError::INTERRUPTED) => {
                        if blk.as_ctx().is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    Err(err) => return Err(err),
                }
            },
            Err(OsError::INTERRUPTED) => {
                if blk.as_ctx().is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) async fn async_receive_msg(soc: &AsyncSocket, mbuf: &mut MsgBuf) -> Result<usize> {
    loop {
        match soc.wait_for_readable().await {
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
