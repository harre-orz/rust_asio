use crate::socket_base::Protocol;
use std::mem::MaybeUninit;
use std::num::TryFromIntError;
use std::time::Duration;

/// An abstract socket option data type.
pub trait SockOpt<P>: Sized
where
    P: Protocol,
{
    const KEY: (i32, i32);
}

/// An abstract set-able socket option data type.
pub trait SetSockOpt<P>: SockOpt<P>
where
    P: Protocol,
{
    fn len(&self) -> usize {
        size_of::<Self>()
    }
}

/// An abstract get-able socket option data type.
pub trait GetSockOpt<P>: SockOpt<P>
where
    P: Protocol,
{
    fn init(uninit: MaybeUninit<Self>, _len: usize) -> Self {
        unsafe { uninit.assume_init() }
    }
}

/// Socket option to allow the socket to be bound to an address that is already in use.
pub struct ReuseAddr(libc::c_int);

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

impl<P> SetSockOpt<P> for ReuseAddr where P: Protocol {}

impl<P> GetSockOpt<P> for ReuseAddr where P: Protocol {}

pub struct ReusePort(libc::c_int);

impl ReusePort {
    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl<P> SetSockOpt<P> for ReusePort where P: Protocol {}

impl<P> GetSockOpt<P> for ReusePort where P: Protocol {}

pub struct SendBufSize(libc::c_int);

impl SendBufSize {
    pub fn new(size: usize) -> Result<Self, TryFromIntError> {
        let size = libc::c_int::try_from(size)?;
        Ok(Self(size))
    }

    pub unsafe fn new_unchecked(size: usize) -> Self {
        assert!(size < libc::c_int::MAX as usize);
        Self(size as libc::c_int)
    }

    pub const fn get(&self) -> usize {
        self.0 as usize
    }
}

impl<P> SetSockOpt<P> for SendBufSize where P: Protocol {}

impl<P> GetSockOpt<P> for SendBufSize where P: Protocol {}

pub struct RecvBufSize(libc::c_int);

impl RecvBufSize {
    pub fn new(size: usize) -> Result<Self, TryFromIntError> {
        Ok(Self(size.try_into()?))
    }

    pub unsafe fn new_unchecked(size: usize) -> Self {
        assert!(size < libc::c_int::MAX as usize);
        Self(size as libc::c_int)
    }

    pub const fn get(&self) -> usize {
        self.0 as usize
    }
}

impl<P> SetSockOpt<P> for RecvBufSize where P: Protocol {}

impl<P> GetSockOpt<P> for RecvBufSize where P: Protocol {}

pub struct KeepAlive(libc::c_int);

impl KeepAlive {
    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl<P> SetSockOpt<P> for KeepAlive where P: Protocol {}

impl<P> GetSockOpt<P> for KeepAlive where P: Protocol {}

pub struct DoNotRoute(libc::c_int);

impl DoNotRoute {
    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl<P> SetSockOpt<P> for DoNotRoute where P: Protocol {}

impl<P> GetSockOpt<P> for DoNotRoute where P: Protocol {}

pub struct Broadcast(libc::c_int);

impl Broadcast {
    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl<P> SetSockOpt<P> for Broadcast where P: Protocol {}

impl<P> GetSockOpt<P> for Broadcast where P: Protocol {}

pub struct Linger(libc::linger);

impl Linger {
    pub fn new(secs: Option<Duration>) -> Result<Self, TryFromIntError> {
        let linger = if let Some(secs) = secs {
            libc::linger {
                l_onoff: 1,
                l_linger: secs.as_secs().try_into()?,
            }
        } else {
            libc::linger {
                l_onoff: 0,
                l_linger: 0,
            }
        };
        Ok(Self(linger))
    }

    pub unsafe fn new_unchecked(secs: Option<Duration>) -> Self {
        let linger = if let Some(secs) = secs {
            assert!(secs.as_secs() < libc::c_int::MAX as u64);
            libc::linger {
                l_onoff: 1,
                l_linger: secs.as_secs() as libc::c_int,
            }
        } else {
            libc::linger {
                l_onoff: 0,
                l_linger: 0,
            }
        };
        Self(linger)
    }

    pub fn get(&self) -> Option<Duration> {
        if self.0.l_onoff != 0 {
            Some(Duration::from_secs(self.0.l_linger as u64))
        } else {
            None
        }
    }
}

impl<P> SetSockOpt<P> for Linger where P: Protocol {}

impl<P> GetSockOpt<P> for Linger where P: Protocol {}

#[cfg(unix)]
mod ffi {
    use super::*;

    impl<P> SockOpt<P> for ReuseAddr
    where
        P: Protocol,
    {
        const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_REUSEADDR);
    }

    impl<P> SockOpt<P> for ReusePort
    where
        P: Protocol,
    {
        const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_REUSEPORT);
    }

    impl<P> SockOpt<P> for SendBufSize
    where
        P: Protocol,
    {
        const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_SNDBUF);
    }

    impl<P> SockOpt<P> for RecvBufSize
    where
        P: Protocol,
    {
        const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_RCVBUF);
    }

    impl<P> SockOpt<P> for KeepAlive
    where
        P: Protocol,
    {
        const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_KEEPALIVE);
    }

    impl<P> SockOpt<P> for DoNotRoute
    where
        P: Protocol,
    {
        const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_DONTROUTE);
    }

    impl<P> SockOpt<P> for Broadcast
    where
        P: Protocol,
    {
        const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_BROADCAST);
    }

    impl<P> SockOpt<P> for Linger
    where
        P: Protocol,
    {
        const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_LINGER);
    }
}

#[cfg(windows)]
mod ffi {
    impl SockOpt for ReuseAddr {
        const KEY: (i32, i32) = (WinSock::SOL_SOCKET, WinSock::SO_REUSEADDR);
    }

    impl SockOpt for ReusePort {
        const KEY: (i32, i32) = (WinSock::SOL_SOCKET, WinSock::SO_REUSEPORT);
    }

    impl SockOpt for SendBufSize {
        const KEY: (i32, i32) = (WinSock::SOL_SOCKET, WinSock::SO_SNDBUF);
    }

    impl SockOpt for RecvBufSize {
        const KEY: (i32, i32) = (WinSock::SOL_SOCKET, WinSock::SO_RCVBUF);
    }
}
