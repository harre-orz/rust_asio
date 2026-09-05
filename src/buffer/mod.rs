mod sbuf;
pub use self::sbuf::{AsyncIoStream, IoStream, StreamBuf, StreamBufMut};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{MsgBuf, MsgBufMut};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{MsgBuf, MsgBufMut};

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use self::windows::{MsgBuf, MsgBufMut};
