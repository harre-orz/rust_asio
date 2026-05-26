use crate::sockaddr::SockLen;
use crate::socket_base::Protocol;
use std::mem::MaybeUninit;
use std::num::TryFromIntError;
use std::time::Duration;
use std::{ptr, slice};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock;

/// An abstract set-able socket option data type.
pub trait SetSockOpt<P>: 'static
where
    P: Protocol,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]);
}

/// An abstract get-able socket option data type.
pub trait GetSockOpt<P>: Sized + 'static
where
    P: Protocol,
{
    fn init(
        pro: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    );
}

/// Socket option to allow the socket to be bound to an address that is already in use.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct ReuseAddr(libc::c_int);

impl ReuseAddr {
    #[cfg(unix)]
    const LEVEL: libc::c_int = libc::SOL_SOCKET;

    #[cfg(windows)]
    const LEVEL: libc::c_int = WinSock::SOL_SOCKET;

    #[cfg(unix)]
    const NAME: libc::c_int = libc::SOL_SOCKET;

    #[cfg(windows)]
    const NAME: libc::c_int = WinSock::SO_REUSEADDR;

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

impl<P> SetSockOpt<P> for ReuseAddr
where
    P: Protocol,
{
    fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for ReuseAddr
where
    P: Protocol,
{
    fn init(
        _: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

#[test]
fn test_reuse_addr() {
    use crate::ip::Tcp;

    let reuse_addr = ReuseAddr::new(false);
    let (l1, n1, _) = reuse_addr.data(Tcp::V4);
    let (l2, n2, _) = ReuseAddr::init(Tcp::V4);
    assert_eq!(l1, l2);
    assert_eq!(n1, n2);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
#[cfg(unix)]
pub struct ReusePort(libc::c_int);

#[cfg(unix)]
impl ReusePort {
    const LEVEL: libc::c_int = libc::SOL_SOCKET;

    const NAME: libc::c_int = libc::SOL_SOCKET;

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
    fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

#[cfg(unix)]
impl<P> GetSockOpt<P> for ReusePort
where
    P: Protocol,
{
    fn init(
        _: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

#[test]
#[cfg(unix)]
fn test_reuse_port() {
    use crate::ip::Tcp;

    let reuse_port = ReusePort::new(false);
    let (l1, n1, _) = reuse_port.data(Tcp::V4);
    let (l2, n2, _) = ReusePort::init(Tcp::V4);
    assert_eq!(l1, l2);
    assert_eq!(n1, n2);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct SendBufSize(libc::c_int);

impl SendBufSize {
    #[cfg(unix)]
    const LEVEL: libc::c_int = libc::SOL_SOCKET;

    #[cfg(windows)]
    const LEVEL: libc::c_int = WinSock::SOL_SOCKET;

    #[cfg(unix)]
    const NAME: libc::c_int = libc::SO_SNDBUF;

    #[cfg(windows)]
    const NAME: libc::c_int = WinSock::SO_SNDBUF;

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
    fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for SendBufSize
where
    P: Protocol,
{
    fn init(
        _: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

#[test]
fn test_send_buf_size() {
    use crate::ip::Tcp;

    let send_bufsize = SendBufSize::new(0).unwrap();
    let (l1, n1, _) = send_bufsize.data(Tcp::V4);
    let (l2, n2, _) = SendBufSize::init(Tcp::V4);
    assert_eq!(l1, l2);
    assert_eq!(n1, n2);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct RecvBufSize(libc::c_int);

impl RecvBufSize {
    #[cfg(unix)]
    const LEVEL: libc::c_int = libc::SOL_SOCKET;

    #[cfg(windows)]
    const LEVEL: libc::c_int = WinSock::SOL_SOCKET;

    #[cfg(unix)]
    const NAME: libc::c_int = libc::SO_RCVBUF;

    #[cfg(windows)]
    const NAME: libc::c_int = WinSock::SO_RCVBUF;

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
    fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for RecvBufSize
where
    P: Protocol,
{
    fn init(
        _: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

#[test]
fn test_recv_buf_size() {
    use crate::ip::Tcp;

    let recv_bufsize = RecvBufSize::new(0).unwrap();
    let (l1, n1, _) = recv_bufsize.data(Tcp::V4);
    let (l2, n2, _) = RecvBufSize::init(Tcp::V4);
    assert_eq!(l1, l2);
    assert_eq!(n1, n2);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct KeepAlive(libc::c_int);

impl KeepAlive {
    #[cfg(unix)]
    const LEVEL: libc::c_int = libc::SOL_SOCKET;

    #[cfg(windows)]
    const LEVEL: libc::c_int = WinSock::SOL_SOCKET;

    #[cfg(unix)]
    const NAME: libc::c_int = libc::SO_KEEPALIVE;

    #[cfg(windows)]
    const NAME: libc::c_int = WinSock::SO_KEEPALIVE;

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
    fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for KeepAlive
where
    P: Protocol,
{
    fn init(
        _: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

#[test]
fn test_keep_alive() {
    use crate::ip::Tcp;

    let keep_alive = KeepAlive::new(false);
    let (l1, n1, _) = keep_alive.data(Tcp::V4);
    let (l2, n2, _) = KeepAlive::init(Tcp::V4);
    assert_eq!(l1, l2);
    assert_eq!(n1, n2);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct DoNotRoute(libc::c_int);

impl DoNotRoute {
    #[cfg(unix)]
    const LEVEL: libc::c_int = libc::SOL_SOCKET;

    #[cfg(windows)]
    const LEVEL: libc::c_int = WinSock::SOL_SOCKET;

    #[cfg(unix)]
    const NAME: libc::c_int = libc::SO_DONTROUTE;

    #[cfg(windows)]
    const NAME: libc::c_int = WinSock::SO_DONTROUTE;

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
    fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for DoNotRoute
where
    P: Protocol,
{
    fn init(
        _: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

#[test]
fn test_do_not_route() {
    use crate::ip::Tcp;

    let do_not_route = DoNotRoute::new(false);
    let (l1, n1, _) = do_not_route.data(Tcp::V4);
    let (l2, n2, _) = DoNotRoute::init(Tcp::V4);
    assert_eq!(l1, l2);
    assert_eq!(n1, n2);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct Broadcast(libc::c_int);

impl Broadcast {
    #[cfg(unix)]
    const LEVEL: libc::c_int = libc::SOL_SOCKET;

    #[cfg(windows)]
    const LEVEL: libc::c_int = WinSock::SOL_SOCKET;

    #[cfg(unix)]
    const NAME: libc::c_int = libc::SO_BROADCAST;

    #[cfg(windows)]
    const NAME: libc::c_int = WinSock::SO_BROADCAST;

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
    fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for Broadcast
where
    P: Protocol,
{
    fn init(
        _: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

#[test]
fn test_broadcat() {
    use crate::ip::Tcp;

    let broadcat = Broadcast::new(false);
    let (l1, n1, _) = broadcat.data(Tcp::V4);
    let (l2, n2, _) = Broadcast::init(Tcp::V4);
    assert_eq!(l1, l2);
    assert_eq!(n1, n2);
}

#[derive(Copy, Clone)]
pub struct Linger(#[cfg(unix)] libc::linger, #[cfg(windows)] WinSock::LINGER);

impl Linger {
    #[cfg(unix)]
    const LEVEL: libc::c_int = libc::SOL_SOCKET;

    #[cfg(windows)]
    const LEVEL: libc::c_int = WinSock::SOL_SOCKET;

    #[cfg(unix)]
    const NAME: libc::c_int = libc::SO_LINGER;

    #[cfg(windows)]
    const NAME: libc::c_int = WinSock::SO_LINGER;

    #[cfg(unix)]
    const fn linger(onoff: bool, linger: i32) -> Self {
        Self(libc::linger {
            l_onoff: if onoff { 1 } else { 0 },
            l_linger: linger,
        })
    }

    #[cfg(windows)]
    const fn linger(onoff: bool, linger: u16) -> Self {
        Self(WinSock::LINGER {
            l_onoff: if onoff { 1 } else { 0 },
            l_linger: linger,
        })
    }

    pub fn new(secs: Option<Duration>) -> Result<Self, TryFromIntError> {
        if let Some(secs) = secs {
            Ok(Self::linger(true, secs.as_secs().try_into()?))
        } else {
            Ok(Self::linger(false, 0))
        }
    }

    #[cfg(unix)]
    pub const unsafe fn new_unchecked(secs: Option<Duration>) -> Self {
        if let Some(secs) = secs {
            assert!(secs.as_secs() < libc::c_int::MAX as u64);
            Self::linger(true, secs.as_secs() as libc::c_int)
        } else {
            Self::linger(false, 0)
        }
    }

    #[cfg(windows)]
    pub const unsafe fn new_unchecked(secs: Option<Duration>) -> Self {
        if let Some(secs) = secs {
            assert!(secs.as_secs() < u16::MAX as u64);
            Self::linger(true, secs.as_secs() as u16)
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
    fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for Linger
where
    P: Protocol,
{
    fn init(
        _: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

#[test]
fn test_linger() {
    use crate::ip::Tcp;

    let linger = Linger::new(None).unwrap();
    let (l1, n1, _) = linger.data(Tcp::V4);
    let (l2, n2, _) = Linger::init(Tcp::V4);
    assert_eq!(l1, l2);
    assert_eq!(n1, n2);
}
