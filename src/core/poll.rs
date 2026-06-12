use super::{EventScheduler, Intr, Deadline};

#[cfg(target_os = "linux")]
mod poll_epoll;
#[cfg(target_os = "linux")]
pub(crate) use self::poll_epoll::{Epoll as Reactor, Event};

#[cfg(target_os = "macos")]
mod poll_kqueue;
#[cfg(target_os = "macos")]
pub(crate) use self::poll_kqueue::{Event, Kqueue as Reactor};

#[cfg(windows)]
mod poll_iocp;
#[cfg(windows)]
pub(crate) use self::poll_iocp::{Event, Iocp as Reactor};
