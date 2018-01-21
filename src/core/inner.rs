use ffi::{AsRawFd, RawFd, SystemError};
use core::{AsIoContext, Fd, IoContext, Perform, ThreadIoContext};

pub struct InnerSocket<P> {
    ctx: IoContext,
    fd: Fd,
    pro: P,
}

impl<P> InnerSocket<P> {
    pub fn new(ctx: &IoContext, fd: RawFd, pro: P) -> Box<Self> {
        let soc = Box::new(InnerSocket {
            ctx: ctx.clone(),
            fd: Fd::socket(fd),
            pro: pro,
        });
        ctx.as_reactor().register_socket(&soc.fd);
        soc
    }

    pub fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.fd.add_read_op(this, op, err)
    }

    pub fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.fd.add_write_op(this, op, err)
    }

    pub fn cancel(&self) {
        self.fd.cancel_ops(&self.ctx)
    }

    pub fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.fd.next_read_op(this)
    }

    pub fn next_write_op(&self, this: &mut ThreadIoContext) {
        self.fd.next_write_op(this)
    }

    pub fn protocol(&self) -> &P {
        &self.pro
    }
}

unsafe impl<P> AsIoContext for InnerSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl<P> AsRawFd for InnerSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl<P> Drop for InnerSocket<P> {
    fn drop(&mut self) {
        self.ctx.as_reactor().deregister_socket(&self.fd)
    }
}
