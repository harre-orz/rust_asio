
mod callstack;
use self::callstack::ThreadCallStack;

mod task;
pub use self::task::{TaskIoContext as IoContextImpl, IoContextWork, ThreadIoContext, Task};

#[cfg(target_os = "macos")] mod kqueue;
#[cfg(target_os = "macos")] pub use self::kqueue::{KqueueReactor as Reactor, KqueueSocket as SocketImpl};


use ffi::SystemError;

use std::io;
use std::cmp::{Eq, PartialEq};
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


pub trait Perform : Send + 'static {
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError);
}
