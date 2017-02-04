use prelude::{Protocol, IoControl, SocketOption, GetSocketOption, SetSocketOption};
use ffi::*;

use std::mem;

pub const MAX_CONNECTIONS: i32 = 126;

/// Possible values which can be passed to the shutdown method.
#[repr(i32)]
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = SHUT_RD,

    /// Indicates that the writing portion of this socket should be shut down.
    Write = SHUT_WR,

    /// Shut down both the reading and writing portions of this socket.
    Both = SHUT_RDWR,
}

impl Into<i32> for Shutdown {
    fn into(self) -> i32 {
        self as i32
    }
}

#[derive(Default, Clone)]
pub struct NonBlockingIo(i32);

impl NonBlockingIo {
    pub fn new(on: bool) -> NonBlockingIo {
        NonBlockingIo(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }
}

impl IoControl for NonBlockingIo {
    type Data = i32;

    #[cfg(unix)] fn name(&self) -> u64 { FIONBIO }
    #[cfg(windows)] fn name(&self) -> i32 { FIONBIO as i32 }

    fn data(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

/// IO control command to get the amount of data that can be read without blocking.
///
/// Implements the FIONREAD IO control command.
///
/// # Examples
/// Gettable the IO control:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::BytesReadable;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let mut bytes = BytesReadable::default();
/// soc.io_control(&mut bytes).unwrap();
/// let sized = bytes.get();
/// ```
#[derive(Default, Clone)]
pub struct BytesReadable(i32);

impl BytesReadable {
    pub fn get(&self) -> usize {
        self.0 as usize
    }
}

impl IoControl for BytesReadable {
    type Data = i32;

    #[cfg(unix)] fn name(&self) -> u64 { FIONREAD }
    #[cfg(windows)] fn name(&self) -> i32 { FIONREAD }

    fn data(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

// #[derive(Default, Clone)]
// pub struct BytesWritten(i32);

// impl BytesWritten {
//     pub fn get(&self) -> usize {
//         self.0 as usize
//     }
// }

// impl IoControl for BytesWritten {
//     type Data = i32;

//     fn name(&self) -> i32 {
//         TIOCOUTQ as i32
//     }

//     fn data(&mut self) -> &mut Self::Data {
//         &mut self.0
//     }
// }

#[derive(Default, Clone)]
pub struct AtMark(i32);

impl AtMark {
    pub fn get(&self) -> bool {
        self.0 != 0
    }
}

impl IoControl for AtMark {
    type Data = i32;

    #[cfg(unix)] fn name(&self) -> u64 { SIOCATMARK }
    #[cfg(windows)] fn name(&self) -> i32 { SIOCATMARK }

    fn data(&mut self) -> &mut Self::Data {
        &mut self.0
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::Broadcast;
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::Broadcast;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt: Broadcast = soc.get_option().unwrap();
/// let is_set: bool = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct Broadcast(i32);

impl Broadcast {
    pub fn new(on: bool) -> Broadcast {
        Broadcast(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P: Protocol> SocketOption<P> for Broadcast {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_BROADCAST
    }
}

impl<P: Protocol> GetSocketOption<P> for Broadcast {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: Protocol> SetSocketOption<P> for Broadcast {
    fn data(&self) -> &Self::Data {
        &self.0
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::Debug;
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::Debug;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: Debug = soc.get_option().unwrap();
/// let is_set: bool = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct Debug(i32);

impl Debug {
    pub fn new(on: bool) -> Debug {
        Debug(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P: Protocol> SocketOption<P> for Debug {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_DEBUG
    }
}

impl<P: Protocol> GetSocketOption<P> for Debug {

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: Protocol> SetSocketOption<P> for Debug {
    fn data(&self) -> &Self::Data {
        &self.0
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::DoNotRoute;
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::DoNotRoute;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt: DoNotRoute = soc.get_option().unwrap();
/// let is_set: bool = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct DoNotRoute(i32);

impl DoNotRoute {
    pub fn new(on: bool) -> DoNotRoute {
        DoNotRoute(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P: Protocol> SocketOption<P> for DoNotRoute {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_DONTROUTE
    }
}

impl<P: Protocol> GetSocketOption<P> for DoNotRoute {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: Protocol> SetSocketOption<P> for DoNotRoute {
    fn data(&self) -> &Self::Data {
        &self.0
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::KeepAlive;
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::KeepAlive;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: KeepAlive = soc.get_option().unwrap();
/// let is_set: bool = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct KeepAlive(i32);

impl KeepAlive {
    pub fn new(on: bool) -> KeepAlive {
        KeepAlive(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P: Protocol> SocketOption<P> for KeepAlive {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_KEEPALIVE
    }
}

impl<P: Protocol> GetSocketOption<P> for KeepAlive {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: Protocol> SetSocketOption<P> for KeepAlive {
    fn data(&self) -> &Self::Data {
        &self.0
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::Linger;
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::Linger;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: Linger = soc.get_option().unwrap();
/// let is_set: Option<u16> = opt.get();
/// ```
#[derive(Clone)]
pub struct Linger(linger);

impl Linger {
    pub fn new(timeout: Option<u16>) -> Linger {
        match timeout {
            Some(timeout)
                => Linger(linger { l_onoff: 1, l_linger: timeout.into() }),
            None
                => Linger(linger { l_onoff: 0, l_linger: 0 })
        }
    }

    pub fn get(&self) -> Option<u16> {
        if (self.0).l_onoff != 0 {
            Some((self.0).l_linger as u16)
        } else {
            None
        }
    }
}

impl Default for Linger {
    fn default() -> Linger {
        unsafe { mem::zeroed() }
    }
}

impl<P: Protocol> SocketOption<P> for Linger {
    type Data = linger;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_LINGER
    }
}

impl<P: Protocol> GetSocketOption<P> for Linger {
    fn data_mut(&mut self) -> &mut Self::Data {
        unsafe { mem::transmute(&mut self.0) }
    }
}

impl<P: Protocol> SetSocketOption<P> for Linger {
    fn data(&self) -> &Self::Data {
        unsafe { mem::transmute(&self.0) }
    }

    fn size(&self) -> usize {
        mem::size_of_val(&self.0)
    }
}

/// Socket option for the receive buffer size of a socket.
///
/// Implements the SOL_SOCKET/SO_RCVBUF socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::RecvBufferSize;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(RecvBufferSize::new(8192)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::RecvBufferSize;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: RecvBufferSize = soc.get_option().unwrap();
/// let size: usize = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct RecvBufferSize(i32);

impl RecvBufferSize {
    pub fn new(size: usize) -> RecvBufferSize {
        RecvBufferSize(size as i32)
    }

    pub fn get(&self) -> usize {
        self.0 as usize
    }

    pub fn set(&mut self, size: usize) {
        self.0 = size as i32
    }
}

impl<P: Protocol> SocketOption<P> for RecvBufferSize {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_RCVBUF
    }
}

impl<P: Protocol> GetSocketOption<P> for RecvBufferSize {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: Protocol> SetSocketOption<P> for RecvBufferSize {
    fn data(&self) -> &Self::Data {
        &self.0
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::RecvLowWatermark;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(RecvLowWatermark::new(1024)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```rust,no_run
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::RecvLowWatermark;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: RecvLowWatermark = soc.get_option().unwrap();
/// let size: usize = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct RecvLowWatermark(i32);

impl RecvLowWatermark {
    pub fn new(size: usize) -> RecvLowWatermark {
        RecvLowWatermark(size as i32)
    }

    pub fn get(&self) -> usize {
        self.0 as usize
    }

    pub fn set(&mut self, size: usize) {
        self.0 = size as i32
    }
}

impl<P: Protocol> SocketOption<P> for RecvLowWatermark {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_RCVLOWAT
    }
}

impl<P: Protocol> GetSocketOption<P> for RecvLowWatermark {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: Protocol> SetSocketOption<P> for RecvLowWatermark {
    fn data(&self) -> &Self::Data {
        &self.0
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::ReuseAddr;
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::ReuseAddr;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpListener::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: ReuseAddr = soc.get_option().unwrap();
/// let is_set: bool = opt.get();
/// ```

#[derive(Default, Clone)]
pub struct ReuseAddr(i32);

impl<P: Protocol> SocketOption<P> for ReuseAddr {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_REUSEADDR
    }
}

impl<P: Protocol> GetSocketOption<P> for ReuseAddr {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: Protocol> SetSocketOption<P> for ReuseAddr {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}

impl ReuseAddr {
    pub fn new(on: bool) -> ReuseAddr {
        ReuseAddr(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::SendBufferSize;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(SendBufferSize::new(8192)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::SendBufferSize;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: SendBufferSize = soc.get_option().unwrap();
/// let size: usize = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct SendBufferSize(i32);

impl<P: Protocol> SocketOption<P> for SendBufferSize {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_SNDBUF
    }
}

impl<P: Protocol> GetSocketOption<P> for SendBufferSize {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: Protocol> SetSocketOption<P> for SendBufferSize {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}

impl SendBufferSize {
    pub fn new(size: usize) -> SendBufferSize {
        SendBufferSize(size as i32)
    }

    pub fn get(&self) -> usize {
        self.0 as usize
    }

    pub fn set(&mut self, size: usize) {
        self.0 = size as i32
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
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::SendLowWatermark;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(SendLowWatermark::new(1024)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```rust,no_run
/// use asyncio::*;
/// use asyncio::ip::*;
/// use asyncio::socket_base::SendLowWatermark;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: SendLowWatermark = soc.get_option().unwrap();
/// let size: usize = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct SendLowWatermark(i32);

impl SendLowWatermark {
    pub fn new(size: usize) -> SendLowWatermark {
        SendLowWatermark(size as i32)
    }

    pub fn get(&self) -> usize {
        self.0 as usize
    }

    pub fn set(&mut self, size: usize) {
        self.0 = size as i32
    }
}

impl<P: Protocol> SocketOption<P> for SendLowWatermark {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_SNDLOWAT
    }
}

impl<P: Protocol> GetSocketOption<P> for SendLowWatermark {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: Protocol> SetSocketOption<P> for SendLowWatermark {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}
