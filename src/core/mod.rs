use ffi::{SystemError};

mod callstack;
use self::callstack::ThreadCallStack;

mod exec;
pub use self::exec::{Exec, IoContext, IoContextWork, ThreadIoContext};

#[cfg(target_os = "macos")]
mod kqueue;

#[cfg(target_os = "macos")]
pub use self::kqueue::{
    PipeIntr as Intr,
    KqueueSocket as InnerSocket,
    KqueueSignal as InnerSignal,
    KqueueReactor as Reactor,
};

// mod null;
// pub use self::null::{NullFd as Fd, NullReactor as Reactor};

mod expiry;
pub use self::expiry::*;

mod timer_queue;
pub use self::timer_queue::*;

// mod socket_impl;
// pub use self::socket_impl::InnerSocket;
//
// mod signal_impl;
// pub use self::signal_impl::InnerSignal;

pub trait Perform: Send + 'static {
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError);
}

pub unsafe trait AsIoContext {
    fn as_ctx(&self) -> &IoContext;
}
