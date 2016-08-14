use std::boxed::FnBox;
pub use std::os::unix::io::{RawFd, AsRawFd};
pub use libc::{c_void, c_int, c_char};
use {IoObject, IoService};

macro_rules! libc_try {
    ($expr:expr) => (match unsafe { $expr } {
        rc if rc >= 0 => rc,
        _ => return Err(::std::io::Error::last_os_error()),
    })
}

extern {
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    fn errno_location() -> *mut c_int;
}

fn errno() -> i32 {
    unsafe { *errno_location() }
}

mod misc;
pub use self::misc::*;

mod unix;
pub use self::unix::*;

pub struct ReactState(pub i32);
const READY: i32 = 0;
const CANCELED: i32 = -1;
pub type ReactHandler = Box<FnBox(*const IoService, ReactState) + Send + 'static>;

#[cfg(all(not(feature = "asio_no_epoll_reactor"), target_os = "linux"))]
pub mod epoll_reactor;

#[cfg(all(not(feature = "asio_no_epoll_reactor"), target_os = "linux"))]
pub use self::epoll_reactor::{
    EpollReactor as Reactor,
    EpollIoActor as IoActor,
    EpollIntrActor as IntrActor,
};

pub trait AsIoActor : IoObject + AsRawFd + 'static {
    fn as_io_actor(&self) -> &IoActor;
}

pub mod timer_queue;
pub use self::timer_queue::{Expiry, ToExpiry, TimerQueue, WaitActor};

pub trait AsWaitActor : IoObject + 'static {
    fn as_wait_actor(&self) -> &WaitActor;
}

#[cfg(target_os = "linux")]
pub mod timerfd_control;

#[cfg(target_os = "linux")]
pub use self::timerfd_control::Control;

pub mod ops;
