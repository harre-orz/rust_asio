#[cfg(target_os = "linux")]
mod epoll;

#[cfg(target_os = "linux")]
pub use self::epoll::Epoll as Reactor;

#[cfg(target_os = "macos")]
mod kqueue;

#[cfg(target_os = "macos")]
pub use self::kqueue::Kqueue as Reactor;

#[cfg(target_os = "windows")]
mod select;

#[cfg(target_os = "windows")]
pub use self::select::Select as Reactor;
