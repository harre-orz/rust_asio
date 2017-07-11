use ffi::RawFd;
use prelude::Socket;

use std::io;
use std::sync::Arc;
use std::time::Duration;

mod task_io;
pub use self::task_io::TaskIoContext as IoContextImpl;

mod pair_box;
pub use self::pair_box::PairBox;

#[derive(Clone)]
pub struct IoContext(Arc<IoContextImpl>);

impl IoContext {
    pub fn new() -> io::Result<Self> {
        IoContextImpl::new()
    }
}

pub struct SocketContext<P> {
    pub ctx: IoContext,
    pub pro: P,
    pub fd: RawFd,
    pub recv_block: bool,
    pub send_block: bool,
    pub recv_timeout: Option<Duration>,
    pub send_timeout: Option<Duration>,
}

impl<P> SocketContext<P> {
    pub fn new(ctx: &IoContext, pro: P, fd: RawFd) -> SocketContext<P> {
        SocketContext {
            ctx: ctx.clone(),
            pro: pro,
            fd: fd,
            recv_block: true,
            send_block: true,
            recv_timeout: None,
            send_timeout: None,
        }
    }
}

pub trait Tx<P> : Socket<P> {
    fn from_ctx(soc: PairBox<SocketContext<P>>) -> Self;
}

pub trait Rx<P> : Socket<P> {
    fn from_ctx(soc: PairBox<SocketContext<P>>) -> Self;
}
