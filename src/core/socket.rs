use ffi::{RawFd, AsRawFd, SystemError};
use core::{IoContext, AsIoContext, ThreadIoContext, Fd, Perform};

pub struct SocketImpl<P>(Box<(IoContext, Fd, P)>);

impl<P> SocketImpl<P> {
    pub fn new(ctx: &IoContext, fd: RawFd, pro: P) -> Self {
        let soc = Box::new((ctx.clone(), Fd::socket(fd), pro));
        ctx.0.reactor.register_socket(&soc.1);
        SocketImpl(soc)
    }

    pub fn add_read_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        (self.0).1.add_read_op(this, op, err)
    }

    pub fn add_write_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        (self.0).1.add_write_op(this, op, err)
    }

    pub fn cancel_ops(&mut self, ctx: &IoContext) {
        (self.0).1.cancel_ops(ctx)
    }

    pub fn next_read_op(&mut self, this: &mut ThreadIoContext) {
        (self.0).1.next_read_op(this)
    }

    pub fn next_write_op(&mut self, this: &mut ThreadIoContext) {
        (self.0).1.next_write_op(this)
    }

    pub fn protocol(&self) -> &P {
        &(self.0).2
    }
}

impl<P> Drop for SocketImpl<P> {
    fn drop(&mut self) {
        ((self.0).0).0.reactor.deregister_socket(&(self.0).1);
    }
}

unsafe impl<P> AsIoContext for SocketImpl<P> {
    fn as_ctx(&self) -> &IoContext {
        let ctx = &(self.0).0;
        if let Some(this) = ThreadIoContext::callstack(ctx) {
            this.as_ctx()
        } else {
            ctx
        }
    }
}

impl<P> AsRawFd for SocketImpl<P> {
    fn as_raw_fd(&self) -> RawFd {
        (self.0).1.as_raw_fd()
    }
}
