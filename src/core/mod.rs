use ffi::RawFd;
use prelude::Socket;

use std::io;
use std::sync::Arc;

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
}

impl<P> SocketContext<P> {
    pub fn getnonblock(&self) -> io::Result<bool> {
        Ok(false)
    }

    pub fn setnonblock(&self, _on: bool) -> io::Result<()> {
        Ok(())
    }
}

pub trait Tx<P> : Socket<P> {
    fn from_ctx(soc: PairBox<SocketContext<P>>) -> Self;
}

pub trait Rx<P> : Socket<P> {
    fn from_ctx(soc: PairBox<SocketContext<P>>) -> Self;
}
