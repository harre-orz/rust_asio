use prelude::{IoControl, SocketOption, GetSocketOption, SetSocketOption};
use ffi::*;


pub const MAX_CONNECTIONS: i32 = 126;


pub use ffi::Shutdown;


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
    fn name(&self) -> u64 {
        FIONBIO as u64
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
    fn name(&self) -> u64 {
        FIONREAD
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

impl<P> SocketOption<P> for Broadcast {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_BROADCAST
    }
}

impl<P> GetSocketOption<P> for Broadcast {}

impl<P> SetSocketOption<P> for Broadcast {}


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

impl<P> SocketOption<P> for Debug {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_DEBUG
    }
}

impl<P> GetSocketOption<P> for Debug {}

impl<P> SetSocketOption<P> for Debug {}


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

impl<P> SocketOption<P> for DoNotRoute {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_DONTROUTE
    }
}

impl<P> GetSocketOption<P> for DoNotRoute {}

impl<P> SetSocketOption<P> for DoNotRoute {}


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

impl<P> SocketOption<P> for KeepAlive {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_KEEPALIVE
    }
}

impl<P> GetSocketOption<P> for KeepAlive {}

impl<P> SetSocketOption<P> for KeepAlive {}


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
            Some(timeout) => Linger(linger {
                l_onoff: 1,
                l_linger: timeout as i32,
            }),
            None => Default::default(),
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

impl Default for Linger {
    fn default() -> Linger {
        Linger(linger {
            l_onoff: 0,
            l_linger: 0,
        })
    }
}

impl<P> SocketOption<P> for Linger {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_LINGER
    }
}

impl<P> GetSocketOption<P> for Linger {}

impl<P> SetSocketOption<P> for Linger {}


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

impl<P> SocketOption<P> for RecvBufferSize {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_RCVBUF
    }
}

impl<P> GetSocketOption<P> for RecvBufferSize {}

impl<P> SetSocketOption<P> for RecvBufferSize {}


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

impl<P> SocketOption<P> for RecvLowWatermark {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_RCVLOWAT
    }
}

impl<P> GetSocketOption<P> for RecvLowWatermark {}

impl<P> SetSocketOption<P> for RecvLowWatermark {}


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

impl<P> SocketOption<P> for ReuseAddr {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_REUSEADDR
    }
}

impl<P> GetSocketOption<P> for ReuseAddr {}

impl<P> SetSocketOption<P> for ReuseAddr {}

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

impl<P> SocketOption<P> for SendBufferSize {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_SNDBUF
    }
}

impl<P> GetSocketOption<P> for SendBufferSize {}

impl<P> SetSocketOption<P> for SendBufferSize {}


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

impl<P> SocketOption<P> for SendLowWatermark {
    fn level(&self, _: &P) -> i32 {
        SOL_SOCKET
    }

    fn name(&self, _: &P) -> i32 {
        SO_SNDLOWAT
    }
}

impl<P> GetSocketOption<P> for SendLowWatermark {}

impl<P> SetSocketOption<P> for SendLowWatermark {}
