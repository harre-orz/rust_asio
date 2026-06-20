use super::{Deadline, Intr, Scheduler};

#[cfg(target_os = "linux")]
mod epoll;
#[cfg(target_os = "linux")]
pub(super) use self::epoll::Epoll as Reactor;
#[cfg(target_os = "linux")]
pub(crate) use self::epoll::{EpollEvent as Event, EpollEventGuard as EventGuard};

#[cfg(target_os = "macos")]
mod kqueue;
#[cfg(target_os = "macos")]
pub(crate) use self::kqueue::{Kevent as Event, KeventGuard as EventGuard};
#[cfg(target_os = "macos")]
pub(super) use self::kqueue::Kqueue as Reactor;

#[cfg(windows)]
mod iocp;
#[cfg(windows)]
pub(super) use self::iocp::Iocp as Reactor;
#[cfg(windows)]
pub(crate) use self::iocp::{IocpEvent as Event, IocpEventGuard as EventGuard};
