mod event;
use self::event::{Deadline, Event, EventScheduler};

#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
mod intr_timerfd;
#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
use self::intr_timerfd::TimerFd as Interrupter;

#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
mod intr_eventfd;
#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
use self::intr_eventfd::EventFd as Interrupter;

#[cfg(any(windows, target_os = "macos", not(any(feature = "timerfd", feature = "eventfd"))))]
mod intr_pipe;
#[cfg(any(windows, target_os = "macos", not(any(feature = "timerfd", feature = "eventfd"))))]
use self::intr_pipe::Pipe as Interrupter;

#[cfg(target_os = "linux")]
mod poll_epoll;
#[cfg(target_os = "linux")]
use self::poll_epoll::Epoll as Reactor;

#[cfg(target_os = "macos")]
mod poll_kqueue;
#[cfg(target_os = "macos")]
use self::poll_kqueue::Kqueue as Reactor;

#[cfg(windows)]
mod poll_iocp;
#[cfg(windows)]
use self::poll_iocp::Iocp as Reactor;

mod context;
pub(crate) use self::context::AsyncSocket;
pub use self::context::IoContext;
