pub trait IntoI32 {
    fn i32(self) -> i32;
}

impl IntoI32 for i32 {
    fn i32(self) -> i32 {
        self
    }
}

#[cfg(unix)] mod posix;
#[cfg(unix)] pub use self::posix::*;

#[cfg(windows)] mod win;
#[cfg(windows)] pub use self::win::*;

mod tss;
pub use self::tss::TssPtr;

mod sa;
pub use self::sa::SockAddrImpl;

mod fdset;
pub use self::fdset::FdSet;
