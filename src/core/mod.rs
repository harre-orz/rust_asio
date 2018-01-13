use ffi::{RawFd, AsRawFd, SystemError};

use std::io;
use std::sync::Arc;


mod callstack;
use self::callstack::ThreadCallStack;

mod task;
pub use self::task::{TaskIoContext as IoContextImpl, IoContextWork, ThreadIoContext, Task};

#[cfg(target_os = "macos")] mod kqueue;
#[cfg(target_os = "macos")] pub use self::kqueue::{KqueueReactor as Reactor, KqueueFd as Fd};

#[cfg(target_os = "macos")] mod pipe;
#[cfg(target_os = "macos")] pub use self::pipe::{PipeIntr as Intr};

mod timer_queue;
pub use self::timer_queue::{Expiry, TimerOp, TimerQueue};


#[derive(Clone)]
pub struct IoContext(Arc<IoContextImpl>);

impl IoContext {
    pub fn new() -> io::Result<Self> {
        IoContextImpl::new()
    }

    #[doc(hidden)]
    pub fn do_dispatch<F: Task>(&self, task: F) {
        IoContextImpl::do_dispatch(self, task)
    }

    #[doc(hidden)]
    pub fn do_perform(&self, op: Box<Perform>, err: SystemError) {
        IoContextImpl::do_perform(self, op, err)
    }

    #[doc(hidden)]
    pub fn do_post<F: Task>(&self, task: F) {
        IoContextImpl::do_post(self, task)
    }

    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(&IoContext) + Send + 'static,
    {
        IoContextImpl::do_dispatch(self, func)
    }

    pub fn post<F>(&self, func: F)
        where F: FnOnce(&IoContext) + Send + 'static,
    {
        IoContextImpl::do_post(self, func)
    }

    pub fn restart(&self) {
        self.0.restart()
    }

    pub fn run(&self) -> usize {
        IoContextImpl::run(self)
    }

    pub fn run_one(&self) -> usize {
        IoContextImpl::run_one(self)
    }

    pub fn stop(&self) {
        IoContextImpl::stop(self)
    }

    pub fn stopped(&self) -> bool {
        self.0.stopped()
    }
}

impl PartialEq for IoContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for IoContext {}


pub unsafe trait AsIoContext {
    fn as_ctx(&self) -> &IoContext;
}

unsafe impl AsIoContext for IoContext {
    fn as_ctx(&self) -> &IoContext {
        self
    }
}


pub trait Perform : Send {
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError);
}

pub trait Perform2 : Perform + Sized + 'static {
}

pub trait AsyncSocket {
    fn add_read_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn add_write_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn next_read_op(&mut self, this: &mut ThreadIoContext);

    fn next_write_op(&mut self, this: &mut ThreadIoContext);
}


pub trait AsyncWaitOp {
    fn add_wait_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn next_wait_op(&mut self, this: &mut ThreadIoContext);
}


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

    pub fn cancel(&mut self) {
        let &mut (ref ctx, ref mut fd, ref pro) = &mut *self.0;
        fd.cancel_ops(ctx)
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
