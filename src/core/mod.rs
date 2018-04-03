mod unsafe_ref;
use self::unsafe_ref::UnsafeRef;

mod callstack;
use self::callstack::ThreadCallStack;

mod exec;
pub use self::exec::{Exec, IoContext, AsIoContext, Perform, IoContextWork, ThreadIoContext};

mod socket_impl;
pub use self::socket_impl::SocketImpl;

mod timer_impl;
pub use self::timer_impl::{Expiry, TimerImpl};

mod simple;
use self::simple::{SimpleTimerQueue as TimerQueue};

#[cfg(target_os = "linux")] mod eventfd;
#[cfg(target_os = "linux")] use self::eventfd::EventFdIntr as Intr;

#[cfg(target_os = "macos")] mod pipe;
#[cfg(target_os = "macos")] use self::pipe::PipeIntr as Intr;

#[cfg(target_os = "linux")] mod epoll;
#[cfg(target_os = "linux")] pub use self::epoll::{Epoll as Handle, EpollReactor as Reactor};

#[cfg(target_os = "macos")] mod kqueue;
#[cfg(target_os = "macos")] pub use self::kqueue::{Kevent as Handle, KqueueReactor as Reactor};
