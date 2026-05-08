mod event;
use self::event::{Deadline, Event, EventScheduler};

#[cfg(all(feature = "intr_timerfd", any(target_os = "linux")))]
mod intr_timerfd;
#[cfg(all(feature = "intr_timerfd", any(target_os = "linux")))]
use self::intr_timerfd::TimerFd as Interrupter;

#[cfg(all(
    feature = "intr_eventfd",
    any(target_os = "linux", target_os = "macos")
))]
mod intr_eventfd;
#[cfg(all(
    feature = "intr_eventfd",
    any(target_os = "linux", target_os = "macos")
))]
use self::intr_eventfd::EventFd as Interrupter;

#[cfg(feature = "intr_pipe")]
mod intr_pipe;
#[cfg(feature = "intr_pipe")]
use self::intr_pipe::Pipe as Interrupter;

#[cfg(all(feature = "poll_epoll", any(target_os = "linux")))]
mod poll_epoll;
#[cfg(all(feature = "poll_epoll", any(target_os = "linux")))]
use self::poll_epoll::Epoll as Reactor;

#[cfg(all(feature = "poll_kqueue", any(target_os = "macos")))]
mod poll_kqueue;
#[cfg(all(feature = "poll_kqueue", any(target_os = "macos")))]
use self::poll_kqueue::Kqueue as Reactor;

#[cfg(feature = "poll_select")]
mod poll_select;
#[cfg(feature = "poll_select")]
use self::poll_select::Select as Reactor;

mod context;
pub(crate) use self::context::AsyncSocket;
pub use self::context::IoContext;
