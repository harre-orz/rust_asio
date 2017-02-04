
// mod null;
// pub use self::null::*;

#[cfg(target_os = "linux")] mod eventfd;
#[cfg(target_os = "linux")] pub use self::eventfd::{
    EventFdInterrupter as Interrupter,
};

#[cfg(target_os = "macos")] mod pipe;
#[cfg(target_os = "macos")] pub use self::pipe::{
    PipeInterrupter as Interrupter,
};

#[cfg(windows)] mod socket;
#[cfg(windows)] pub use self::socket::{
    SocketInterrupter as Interrupter,
};
