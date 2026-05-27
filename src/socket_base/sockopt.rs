use crate::sockaddr::SockLen;
use crate::socket_base::Protocol;
use std::mem::MaybeUninit;
use std::num::TryFromIntError;
use std::time::Duration;
use std::{ptr, slice};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock;

pub struct SockOpt {
    pub level: libc::c_int,
    pub name: libc::c_int,
}

/// An abstract set-able socket option data type.
pub trait SetSockOpt<P>: 'static
where
    P: Protocol,
{
    fn data(&self, pro: P) -> (SockOpt, &[u8]);
}

/// An abstract get-able socket option data type.
pub trait GetSockOpt<P>: Sized + 'static
where
    P: Protocol,
{
    fn init(pro: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self);
}

/// Socket option to allow the socket to be bound to an address that is already in use.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct ReuseAddr(libc::c_int);

impl ReuseAddr {
    const KEY: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::SOL_SOCKET,
        #[cfg(unix)]
        name: libc::SO_REUSEADDR,

        #[cfg(windows)]
        level: WinSock::SOL_SOCKET,
        #[cfg(windows)]
        name: WinSock::SO_REUSEADDR,
    };

    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(self) -> bool {
        self.0 != 0
    }

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for ReuseAddr
where
    P: Protocol,
{
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for ReuseAddr
where
    P: Protocol,
{
    fn init(_: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
#[cfg(unix)]
pub struct ReusePort(libc::c_int);

#[cfg(unix)]
impl ReusePort {
    const KEY: SockOpt = SockOpt {
        level: libc::SOL_SOCKET,
        name: libc::SOL_SOCKET,
    };

    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

#[cfg(unix)]
impl<P> SetSockOpt<P> for ReusePort
where
    P: Protocol,
{
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

#[cfg(unix)]
impl<P> GetSockOpt<P> for ReusePort
where
    P: Protocol,
{
    fn init(_: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct SendBufSize(libc::c_int);

impl SendBufSize {
    const KEY: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::SOL_SOCKET,
        #[cfg(unix)]
        name: libc::SO_SNDBUF,

        #[cfg(windows)]
        level: WinSock::SOL_SOCKET,
        #[cfg(windows)]
        name: WinSock::SO_SNDBUF,
    };

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

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for SendBufSize
where
    P: Protocol,
{
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for SendBufSize
where
    P: Protocol,
{
    fn init(_: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct RecvBufSize(libc::c_int);

impl RecvBufSize {
    const KEY: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::SOL_SOCKET,
        #[cfg(unix)]
        name: libc::SO_RCVBUF,

        #[cfg(windows)]
        level: WinSock::SOL_SOCKET,
        #[cfg(windows)]
        name: WinSock::SO_RCVBUF,
    };

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

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for RecvBufSize
where
    P: Protocol,
{
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for RecvBufSize
where
    P: Protocol,
{
    fn init(_: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct KeepAlive(libc::c_int);

impl KeepAlive {
    const KEY: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::SOL_SOCKET,
        #[cfg(unix)]
        name: libc::SO_KEEPALIVE,

        #[cfg(windows)]
        level: WinSock::SOL_SOCKET,
        #[cfg(windows)]
        name: WinSock::SO_KEEPALIVE,
    };

    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for KeepAlive
where
    P: Protocol,
{
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for KeepAlive
where
    P: Protocol,
{
    fn init(_: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct DoNotRoute(libc::c_int);

impl DoNotRoute {
    const KEY: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::SOL_SOCKET,
        #[cfg(unix)]
        name: libc::SO_DONTROUTE,

        #[cfg(windows)]
        level: WinSock::SOL_SOCKET,
        #[cfg(windows)]
        name: WinSock::SO_DONTROUTE,
    };

    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for DoNotRoute
where
    P: Protocol,
{
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for DoNotRoute
where
    P: Protocol,
{
    fn init(_: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct Broadcast(libc::c_int);

impl Broadcast {
    const KEY: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::SOL_SOCKET,
        #[cfg(unix)]
        name: libc::SO_BROADCAST,

        #[cfg(windows)]
        level: WinSock::SOL_SOCKET,
        #[cfg(windows)]
        name: WinSock::SO_BROADCAST,
    };

    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for Broadcast
where
    P: Protocol,
{
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for Broadcast
where
    P: Protocol,
{
    fn init(_: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}

#[derive(Copy, Clone)]
pub struct Linger(#[cfg(unix)] libc::linger, #[cfg(windows)] WinSock::LINGER);

impl Linger {
    const KEY: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::SOL_SOCKET,
        #[cfg(unix)]
        name: libc::SO_LINGER,

        #[cfg(windows)]
        level: WinSock::SOL_SOCKET,
        #[cfg(windows)]
        name: WinSock::SO_LINGER,
    };

    const fn linger(onoff: bool, linger: i32) -> Self {
        #[cfg(unix)]
        let linger = libc::linger {
            l_onoff: if onoff { 1 } else { 0 },
            l_linger: linger,
        };
        #[cfg(windows)]
        let linger = WinSock::LINGER {
            l_onoff: if onoff { 1 } else { 0 },
            l_linger: linger as u16,
        };
        Self(linger)
    }

    pub fn new(secs: Option<Duration>) -> Result<Self, TryFromIntError> {
        if let Some(secs) = secs {
            Ok(Self::linger(true, secs.as_secs().try_into()?))
        } else {
            Ok(Self::linger(false, 0))
        }
    }

    pub const unsafe fn new_unchecked(secs: Option<Duration>) -> Self {
        if let Some(secs) = secs {
            assert!(secs.as_secs() < libc::c_int::MAX as u64);
            Self::linger(true, secs.as_secs() as libc::c_int)
        } else {
            Self::linger(false, 0)
        }
    }

    pub const fn get(&self) -> Option<Duration> {
        if self.0.l_onoff != 0 {
            Some(Duration::from_secs(self.0.l_linger as u64))
        } else {
            None
        }
    }

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for Linger
where
    P: Protocol,
{
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for Linger
where
    P: Protocol,
{
    fn init(_: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}
