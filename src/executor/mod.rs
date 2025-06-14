mod event;
use self::event::{Event, EventScheduler};

// intr

#[cfg(target_os = "linux")]
mod intr_timerfd;

#[cfg(target_os = "linux")]
use self::intr_timerfd::TimerFd as Intr;

#[cfg(target_os = "macos")]
mod intr_eventfd;

#[cfg(target_os = "macos")]
use self::intr_eventfd::EventFd as Intr;

#[cfg(target_os = "windows")]
mod intr_pair;

#[cfg(target_os = "windows")]
use self::intr_pair::SocketPair as Intr;

// reactor

#[cfg(target_os = "linux")]
mod reactor_epoll;

#[cfg(target_os = "linux")]
use self::reactor_epoll::Epoll as Reactor;

#[cfg(target_os = "macos")]
mod reactor_kqueue;

#[cfg(target_os = "macos")]
use self::reactor_kqueue::Kqueue as Reactor;

#[cfg(target_os = "windows")]
mod reactor_select;

#[cfg(target_os = "windows")]
use self::reactor_select::Select as Reactor;

// context

mod io_context;
pub use self::io_context::IoContext;

mod async_socket;
pub use self::async_socket::AsyncSocket;
