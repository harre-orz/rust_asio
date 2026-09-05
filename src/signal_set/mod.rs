#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{Signal, AsyncSignalSet, SignalSet};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{Signal, AsyncSignalSet, SignalSet};
