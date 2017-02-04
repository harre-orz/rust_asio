mod fd;
pub use self::fd::{
    Dispatcher, FdContext, Ops, IntrFd, AsyncFd,
};

// mod null;
// pub use self::null::*;

#[cfg(target_os = "linux")] mod epoll;
#[cfg(target_os = "linux")] pub use self::epoll::{
    Dispatch,
    EpollReactor as Reactor,
};

#[cfg(target_os = "macos")] mod kqueue;
#[cfg(target_os = "macos")] pub use self::kqueue::{
    Dispatch,
    KqueueReactor as Reactor,
};

#[cfg(windows)] mod select;
#[cfg(windows)] pub use self::select::{
    Dispatch,
    SelectReactor as Reactor,
};
