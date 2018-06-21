#[cfg(unix)]
mod posix;
#[cfg(unix)]
pub use self::posix::*;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use self::win::*;
