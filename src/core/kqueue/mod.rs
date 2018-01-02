use super::{IoContext, AsIoContext, ThreadIoContext, Task, Perform};
use prelude::{Protocol, Socket};
use ffi::{RawFd, AsRawFd, SystemError};

pub struct KqueueReactor;


pub struct KqueueSocket<P> {
    ctx: IoContext,
    pro: P,
    soc: RawFd,
}

impl<P> KqueueSocket<P>
    where P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P, soc: RawFd) -> Box<Self> {
        box unsafe { Self::from_raw_fd(ctx, pro, soc) }
    }
}

impl<P> KqueueSocket<P> {
    pub fn register_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
    }

    pub fn register_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
    }

    pub fn unregister_read_op(&self, this: &mut ThreadIoContext) {
    }

    pub fn unregister_write_op(&self, this: &mut ThreadIoContext) {
    }
}

unsafe impl<P> AsIoContext for KqueueSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl<P> AsRawFd for KqueueSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc
    }
}

impl<P> Socket<P> for KqueueSocket<P>
    where P: Protocol,
{
    fn protocol(&self) -> &P {
        &self.pro
    }

    unsafe fn from_raw_fd(ctx: &IoContext, pro: P, soc: RawFd) -> Self {
        KqueueSocket {
            ctx: ctx.clone(),
            pro: pro,
            soc: soc,
        }
    }
}
