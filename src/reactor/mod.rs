mod socket_impl;
pub use self::socket_impl::SocketImpl;

#[cfg(target_os = "linux")]
mod eventfd;
#[cfg(target_os = "linux")]
use self::eventfd::EventFdIntr as Intr;

#[cfg(target_os = "macos")]
mod pipe;
#[cfg(target_os = "macos")]
use self::pipe::PipeIntr as Intr;

#[cfg(target_os = "linux")]
mod epoll;
#[cfg(target_os = "linux")]
pub use self::epoll::{Epoll as Handle, EpollReactor as Reactor};

#[cfg(target_os = "macos")]
mod kqueue;
#[cfg(target_os = "macos")]
pub use self::kqueue::{Kevent as Handle, KqueueReactor as Reactor};
