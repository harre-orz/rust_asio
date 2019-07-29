//

use executor::{AsIoContext, IoContext, SocketContext};
use libc;
use std::fmt;
use std::mem;

pub const MAX_CONNECTIONS: i32 = 126;

#[cfg(unix)]
pub type NativeHandle = std::os::unix::io::RawFd;

#[cfg(windows)]
pub type NativeHandle = std::os::windows::raw::SOCKET;

pub trait Endpoint<P> {
    fn as_ptr(&self) -> *const libc::sockaddr;
    fn as_mut_ptr(&mut self) -> *mut libc::sockaddr;
    fn capacity(&self) -> libc::socklen_t;
    fn size(&self) -> libc::socklen_t;
    unsafe fn resize(&mut self, libc::socklen_t);
}

pub trait Socket<P>: AsIoContext {
    #[doc(hidden)]
    fn as_inner(&self) -> &SocketContext;
    fn native_handle(&self) -> NativeHandle;
    unsafe fn unsafe_new(ctx: &IoContext, pro: P, soc: NativeHandle) -> Self;
}

pub trait Protocol: Sized {
    type Endpoint: Endpoint<Self>;
    type Socket: Socket<Self>;
    fn family_type(&self) -> i32;
    fn socket_type(&self) -> i32;
    fn protocol_type(&self) -> i32;
    unsafe fn uninitialized(&self) -> Self::Endpoint;
}

pub trait IoControl {
    fn name(&self) -> u64;

    fn as_ptr(&self) -> *const libc::c_void {
        self as *const _ as *const _
    }

    fn as_mut_ptr(&mut self) -> *mut libc::c_void {
        self as *mut _ as *mut _
    }
}

pub trait GetSocketOption<P> {
    fn get_sockopt(
        &mut self,
        pro: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)>;

    unsafe fn resize(&mut self, _len: libc::socklen_t) {}
}

pub trait SetSocketOption<P> {
    fn set_sockopt(
        &self,
        pro: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )>;
}

pub fn get_sockopt<T>(
    level: i32,
    name: i32,
    data: &mut T,
) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
    Some((
        level,
        name,
        data as *mut _ as *mut _,
        mem::size_of::<T>() as _,
    ))
}

pub const fn set_sockopt<T>(
    level: i32,
    name: i32,
    data: &T,
) -> Option<(
    libc::c_int,
    libc::c_int,
    *const libc::c_void,
    libc::socklen_t,
)> {
    Some((
        level,
        name,
        data as *const _ as *const _,
        mem::size_of::<T>() as _,
    ))
}

#[repr(i32)]
pub enum Shutdown {
    Read = libc::SHUT_RD,
    Write = libc::SHUT_WR,
    Both = libc::SHUT_RDWR,
}

/// IO control command to get the amount of data that can be read without blocking.
///
/// Implements the FIONREAD IO control command.
///
/// # Examples
/// Gettable the IO control:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::BytesReadable;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let mut bytes = BytesReadable::new();
/// soc.io_control(&mut bytes).unwrap();
/// assert_eq!(bytes.get(), 0)
/// ```
#[derive(Clone, Debug)]
pub struct BytesReadable(i32);

impl BytesReadable {
    pub const fn new() -> Self {
        BytesReadable(-1)
    }
    pub const fn get(&self) -> usize {
        self.0 as usize
    }
}

impl IoControl for BytesReadable {
    fn name(&self) -> u64 {
        libc::FIONREAD
    }
}

/// socket option to permit sending of broadcast messages.
///
/// Implements the SOL_SOCKET/SO_BROADCAST socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::Broadcast;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// soc.set_option(Broadcast::new(true)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::Broadcast;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt: Broadcast = soc.get_option().unwrap();
/// assert_eq!(opt.get(), false)
/// ```
#[derive(Clone, Debug)]
pub struct Broadcast(i32);

impl Broadcast {
    pub const fn new(on: bool) -> Self {
        Broadcast(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P> GetSocketOption<P> for Broadcast {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_BROADCAST, self)
    }
}

impl<P> SetSocketOption<P> for Broadcast {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_BROADCAST, self)
    }
}

/// Socket option to enable socket-level debugging.
///
/// Implements the SOL_SOCKET/SO_DEBUG socket option.
///
/// # Examples
/// Setting the option:
///
/// ```rust,no_run
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::Debug;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(Debug::new(true)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::Debug;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: Debug = soc.get_option().unwrap();
/// assert_eq!(opt.get(), false)
/// ```
#[derive(Clone, Debug)]
pub struct Debug(i32);

impl Debug {
    pub const fn new(on: bool) -> Self {
        Debug(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P> GetSocketOption<P> for Debug {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_DEBUG, self)
    }
}

impl<P> SetSocketOption<P> for Debug {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_DEBUG, self)
    }
}

/// Socket option to don't use a gateway. send to local network host only.
///
/// Implements the SOL_SOCKET/SO_DONTROUTE socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::DoNotRoute;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// soc.set_option(DoNotRoute::new(true)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::DoNotRoute;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt: DoNotRoute = soc.get_option().unwrap();
/// assert_eq!(opt.get(), false)
/// ```
#[derive(Clone, Debug)]
pub struct DoNotRoute(i32);

impl DoNotRoute {
    pub const fn new(on: bool) -> Self {
        DoNotRoute(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P> GetSocketOption<P> for DoNotRoute {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_DONTROUTE, self)
    }
}

impl<P> SetSocketOption<P> for DoNotRoute {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_DONTROUTE, self)
    }
}

/// Socket option to send keep-alives.
///
/// Implements the SOL_SOKCET/SO_KEEPALIVE socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::KeepAlive;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(KeepAlive::new(true)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::KeepAlive;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: KeepAlive = soc.get_option().unwrap();
/// assert_eq!(opt.get(), false)
/// ```
#[derive(Clone, Debug)]
pub struct KeepAlive(i32);

impl KeepAlive {
    pub const fn new(on: bool) -> Self {
        KeepAlive(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P> GetSocketOption<P> for KeepAlive {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_KEEPALIVE, self)
    }
}

impl<P> SetSocketOption<P> for KeepAlive {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_KEEPALIVE, self)
    }
}

/// Socket option to specify whether the socket lingers on close if unsent data is present.
///
/// Implements the SOL_SOCKET/SO_LINGER socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::Linger;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(Linger::new(Some(30))).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::Linger;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: Linger = soc.get_option().unwrap();
/// assert_eq!(opt.get(), None)
/// ```
#[derive(Clone)]
pub struct Linger(libc::linger);

impl Linger {
    pub fn new(timeout: Option<u16>) -> Self {
        match timeout {
            Some(timeout) => Linger(libc::linger {
                l_onoff: 1,
                l_linger: timeout as i32,
            }),
            None => Linger(libc::linger {
                l_onoff: 0,
                l_linger: 0,
            }),
        }
    }

    pub fn get(&self) -> Option<u16> {
        if self.0.l_onoff != 0 {
            Some(self.0.l_linger as u16)
        } else {
            None
        }
    }
}

impl fmt::Debug for Linger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Linger {{ l_onoff = {}, l_linger = {} }}",
            self.0.l_onoff, self.0.l_linger
        )
    }
}

impl<P> GetSocketOption<P> for Linger {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_LINGER, self)
    }
}

impl<P> SetSocketOption<P> for Linger {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_LINGER, self)
    }
}

///Socket option for the receive buffer size of a socket.
///
/// Implements the SOL_SOCKET/SO_RCVBUF socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::RecvBufSize;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(RecvBufSize::new(8192)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::RecvBufSize;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: RecvBufSize = soc.get_option().unwrap();
/// assert!(opt.get() > 0)
/// ```
#[derive(Clone, Debug)]
pub struct RecvBufSize(i32);

impl RecvBufSize {
    pub const fn new(size: usize) -> Self {
        RecvBufSize(size as i32)
    }

    pub const fn get(&self) -> usize {
        self.0 as usize
    }

    pub fn set(&mut self, size: usize) {
        self.0 = size as i32
    }
}

impl<P> GetSocketOption<P> for RecvBufSize {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_RCVBUF, self)
    }
}

impl<P> SetSocketOption<P> for RecvBufSize {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_RCVBUF, self)
    }
}

/// Socket option for the receive low watermark.
///
/// Implements the SOL_SOCKET/SO_RCVLOWAT socket option.
///
/// # Examples
/// Setting the option:
///
/// ```rust,no_run
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::RecvLowWaterMark;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(RecvLowWaterMark::new(1024)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```rust,no_run
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::RecvLowWaterMark;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: RecvLowWaterMark = soc.get_option().unwrap();
/// assert!(opt.get() > 0)
/// ```
#[derive(Clone, Debug)]
pub struct RecvLowWaterMark(i32);

impl RecvLowWaterMark {
    pub fn new(size: usize) -> Self {
        RecvLowWaterMark(size as i32)
    }

    pub fn get(&self) -> usize {
        self.0 as usize
    }

    pub fn set(&mut self, size: usize) {
        self.0 = size as i32
    }
}

impl<P> GetSocketOption<P> for RecvLowWaterMark {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_RCVLOWAT, self)
    }
}

impl<P> SetSocketOption<P> for RecvLowWaterMark {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_RCVLOWAT, self)
    }
}

/// Socket option to allow the socket to be bound to an address that is already in use.
///
/// Implements the SOL_SOCKET/SO_REUSEADDR socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::ReuseAddr;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpListener::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(ReuseAddr::new(true)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::ReuseAddr;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpListener::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: ReuseAddr = soc.get_option().unwrap();
/// assert_eq!(opt.get(), false)
/// ```
#[derive(Clone, Debug)]
pub struct ReuseAddr(i32);

impl ReuseAddr {
    pub const fn new(on: bool) -> ReuseAddr {
        ReuseAddr(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P> GetSocketOption<P> for ReuseAddr {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_REUSEADDR, self)
    }
}

impl<P> SetSocketOption<P> for ReuseAddr {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_REUSEADDR, self)
    }
}

/// Socket option for the send buffer size of a socket.
///
/// Implements the SOL_SOCKET/SO_SNDBUF socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::SendBufSize;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(SendBufSize::new(8192)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::SendBufSize;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: SendBufSize = soc.get_option().unwrap();
/// assert!(opt.get() > 0)
/// ```
#[derive(Clone, Debug)]
pub struct SendBufSize(i32);

impl SendBufSize {
    pub fn new(size: usize) -> SendBufSize {
        SendBufSize(size as i32)
    }

    pub fn get(&self) -> usize {
        self.0 as usize
    }

    pub fn set(&mut self, size: usize) {
        self.0 = size as i32
    }
}

impl<P> GetSocketOption<P> for SendBufSize {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_SNDBUF, self)
    }
}

impl<P> SetSocketOption<P> for SendBufSize {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_SNDBUF, self)
    }
}

/// Socket option for the send low watermark.
///
/// Implements the SOL_SOCKET/SO_SNDLOWAT socket option.
///
/// # Examples
/// Setting the option:
///
/// ```rust,no_run
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::SendLowWaterMark;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(SendLowWaterMark::new(1024)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```rust,no_run
/// use asyio::*;
/// use asyio::ip::*;
/// use asyio::socket_base::SendLowWaterMark;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: SendLowWaterMark = soc.get_option().unwrap();
/// assert!(opt.get() > 0)
/// ```
#[derive(Clone, Debug)]
pub struct SendLowWaterMark(i32);

impl SendLowWaterMark {
    pub const fn new(size: usize) -> Self {
        SendLowWaterMark(size as i32)
    }

    pub const fn get(&self) -> usize {
        self.0 as usize
    }

    pub fn set(&mut self, size: usize) {
        self.0 = size as i32
    }
}

impl<P> GetSocketOption<P> for SendLowWaterMark {
    fn get_sockopt(
        &mut self,
        _: &P,
    ) -> Option<(libc::c_int, libc::c_int, *mut libc::c_void, libc::socklen_t)> {
        get_sockopt(libc::SOL_SOCKET, libc::SO_SNDLOWAT, self)
    }
}

impl<P> SetSocketOption<P> for SendLowWaterMark {
    fn set_sockopt(
        &self,
        _: &P,
    ) -> Option<(
        libc::c_int,
        libc::c_int,
        *const libc::c_void,
        libc::socklen_t,
    )> {
        set_sockopt(libc::SOL_SOCKET, libc::SO_SNDLOWAT, self)
    }
}

#[test]
fn test_sockopt() {
    trait SocketOption<P>: GetSocketOption<P> + SetSocketOption<P> + fmt::Debug {}

    impl<P, T> SocketOption<P> for T where T: GetSocketOption<P> + SetSocketOption<P> + fmt::Debug {}

    let vec: Vec<Box<dyn SocketOption<_>>> = vec![
        Box::new(Broadcast::new(false)),
        Box::new(Debug::new(false)),
        Box::new(DoNotRoute::new(false)),
        Box::new(KeepAlive::new(false)),
        Box::new(Linger::new(None)),
        Box::new(RecvBufSize::new(0)),
        Box::new(RecvLowWaterMark::new(0)),
        Box::new(ReuseAddr::new(false)),
        Box::new(SendBufSize::new(0)),
        Box::new(SendLowWaterMark::new(0)),
    ];
    vec.into_iter().for_each(|mut x: Box<dyn SocketOption<_>>| {
        let get = x.get_sockopt(&0).unwrap();
        let set = x.set_sockopt(&0).unwrap();
        println!("{:?}", x);
        assert_eq!(get.0, set.0);
        assert_eq!(get.1, set.1);
        assert_eq!(get.2, set.2 as _);
        assert_eq!(get.3, set.3);
    })
}
