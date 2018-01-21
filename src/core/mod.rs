use ffi::SystemError;

mod callstack;
use self::callstack::ThreadCallStack;

mod exec;
pub use self::exec::{Exec, IoContext, IoContextWork, ThreadIoContext};

#[cfg(target_os = "macos")] mod kqueue;
#[cfg(target_os = "macos")] pub use self::kqueue::{KqueueReactor as Reactor, KqueueFd as Fd};

#[cfg(target_os = "macos")] mod pipe;
#[cfg(target_os = "macos")] pub use self::pipe::{PipeIntr as Intr};

// mod null;
// pub use self::null::{NullFd as Fd, NullReactor as Reactor};

mod expiry;
pub use self::expiry::*;

mod inner;
pub use self::inner::*;

mod timer_queue;
pub use self::timer_queue::*;

pub trait Perform: Send + 'static {
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError);
}

pub unsafe trait AsIoContext {
    fn as_ctx(&self) -> &IoContext;
}
