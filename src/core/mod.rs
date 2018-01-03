
mod callstack;
use self::callstack::ThreadCallStack;

mod task;
pub use self::task::{TaskIoContext as IoContextImpl, IoContextWork, ThreadIoContext, Task};

#[cfg(target_os = "macos")] mod kqueue;
#[cfg(target_os = "macos")] pub use self::kqueue::{KqueueReactor as Reactor, KqueueFd as SocketImpl};

// #[cfg(target_os = "macos")] mod pipe;
// #[cfg(target_os = "macos")] pub use self::pipe::{PipeIntr as Intr};


use ffi::SystemError;

use std::io;
use std::sync::Arc;


#[derive(Clone)]
pub struct IoContext(Arc<IoContextImpl>);

impl IoContext {
    pub fn new() -> io::Result<Self> {
        IoContextImpl::new()
    }

    pub fn do_dispatch<F: Task>(&self, task: F) {
        IoContextImpl::do_dispatch(self, task)
    }

    pub fn do_post<F: Task>(&self, task: F) {
        IoContextImpl::do_post(self, task)
    }

    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(&IoContext) + Send + 'static,
    {
        IoContextImpl::dispatch(self, func)
    }

    pub fn post<F>(&self, func: F)
        where F: FnOnce(&IoContext) + Send + 'static,
    {
        IoContextImpl::post(self, func)
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


pub trait Yield<T> {
    fn yield_return(self, ctx: &IoContext) -> T;
}


pub trait Perform {
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError);
}


pub trait AsyncSocket {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn cancel_read_ops(&self, this: &mut ThreadIoContext);

    fn cancel_write_ops(&self, this: &mut ThreadIoContext);

    fn next_read_op(&self, this: &mut ThreadIoContext);

    fn next_write_op(&self, this: &mut ThreadIoContext);
}
