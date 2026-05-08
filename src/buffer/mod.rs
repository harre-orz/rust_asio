#[derive(Debug)]
#[non_exhaustive]
pub struct ReserveError;

mod stream;
pub use self::stream::{AsyncIoStream, IoStream, StreamBuf, StreamBufMut};

mod msg;
pub use self::msg::MsgBufMut;

#[cfg(target_os = "linux")]
mod msg_linux;
#[cfg(target_os = "linux")]
pub use self::msg_linux::MsgBuf;

#[cfg(target_os = "macos")]
mod msg_macos;
#[cfg(target_os = "macos")]
pub use self::msg_macos::MsgBuf;

#[cfg(target_os = "windows")]
mod msg_windows;
#[cfg(target_os = "windows")]
pub use self::msg_windows::MsgBuf;
