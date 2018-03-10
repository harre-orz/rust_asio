mod unsafe_ref;
pub use self::unsafe_ref::UnsafeRef;

mod callstack;
use self::callstack::ThreadCallStack;

mod exec;
pub use self::exec::{Exec, IoContext, AsIoContext, Perform, IoContextWork, ThreadIoContext};

mod timer_impl;
pub use self::timer_impl::{Expiry, TimerImpl, TimerQueue};

#[cfg(target_os = "macos")]
mod pipe;
#[cfg(target_os = "macos")]
pub use self::pipe::PipeIntr as Intr;

#[cfg(target_os = "macos")]
mod kqueue;
#[cfg(target_os = "macos")]
pub use self::kqueue::{Kevent as Handle, KqueueReactor as Reactor};

mod socket_impl;
pub use self::socket_impl::SocketImpl;

#[cfg(unix)]
mod signal_impl;
#[cfg(unix)]
pub use self::signal_impl::SignalImpl;
