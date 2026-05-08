use std::mem::MaybeUninit;

/// An abstract socket option data type.
pub trait SockOpt: Sized {
    const KEY: (i32, i32);
}

/// An abstract set-able socket option data type.
pub trait SetSockOpt: SockOpt {
    fn len(&self) -> usize {
        size_of::<Self>()
    }
}

/// An abstract get-able socket option data type.
pub trait GetSockOpt: SockOpt {
    fn init(opt: MaybeUninit<Self>, _len: usize) -> Self {
        unsafe { opt.assume_init() }
    }
}

/// Socket option to allow the socket to be bound to an address that is already in use.
pub struct ReuseAddr(i32);

impl ReuseAddr {
    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl SetSockOpt for ReuseAddr {}

impl GetSockOpt for ReuseAddr {}

#[cfg(unix)]
mod ffi {
    use super::*;

    impl SockOpt for ReuseAddr {
        const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_REUSEADDR);
    }
}

#[cfg(windows)]
mod ffi {
    impl SockOpt for ReuseAddr {
        const KEY: (i32, i32) = (WinSock::SOL_SOCKET, WinSock::SO_REUSEADDR);
    }
}
