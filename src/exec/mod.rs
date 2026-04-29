mod event;
use self::event::{Event, EventScheduler};

#[cfg(target_os = "linux")]
mod intr_timerfd;
#[cfg(target_os = "linux")]
use self::intr_timerfd::TimerFd as Interrupter;

#[cfg(target_os = "macos")]
mod poll_kqueue;
#[cfg(target_os = "macos")]
use self::poll_kqueue::Kqueue as Reactor;

#[cfg(target_os = "windows")]
mod intr_pipe;
#[cfg(target_os = "windows")]
use self::intr_pipe::SocketPair as Interrupter;

#[cfg(target_os = "linux")]
mod poll_epoll;
#[cfg(target_os = "linux")]
use self::poll_epoll::Epoll as Reactor;

#[cfg(target_os = "macos")]
mod poll_kqueue;
#[cfg(target_os = "macos")]
use self::poll_kqueue::Kqueue as Reactor;

#[cfg(target_os = "windows")]
mod poll_select;
#[cfg(target_os = "windows")]
use self::poll_select::Select as Reactor;

mod context;
pub(crate) use self::context::AsyncSocket;
pub use self::context::IoContext;
