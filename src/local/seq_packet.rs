use ffi::{AF_UNIX, SOCK_SEQPACKET, sockaddr, socklen_t};
use prelude::{Endpoint, Protocol};
use socket_base::{Tx, Rx};
use socket_builder::SocketBuilder;
use socket_listener::SocketListener;
use dgram_socket::DgramSocket;
use local::{LocalEndpoint, LocalProtocol};

use std::fmt;
use std::mem;

/// The seq-packet protocol.
///
/// # Example
/// Create a server and client sockets.
///
/// ```rust,no_run
/// use asyncio::{IoContext, Endpoint};
/// use asyncio::local::{LocalSeqPacket, LocalSeqPacketEndpoint, LocalSeqPacketSocket, LocalSeqPacketListener};
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = LocalSeqPacketEndpoint::new("example.sock").unwrap();
///
/// let sv = LocalSeqPacketListener::new(ctx, LocalSeqPacket).unwrap();
/// sv.bind(&ep).unwrap();
/// sv.listen().unwrap();
///
/// let cl = LocalSeqPacketSocket::new(ctx, ep.protocol()).unwrap();
/// cl.connect(&ep).unwrap();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct LocalSeqPacket;

impl LocalEndpoint<LocalSeqPacket> {
    pub fn protocol(&self) -> LocalSeqPacket {
        LocalSeqPacket
    }
}

impl Protocol for LocalSeqPacket {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_UNIX
    }

    fn socket_type(&self) -> i32 {
        SOCK_SEQPACKET
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl LocalProtocol for LocalSeqPacket {
    type Tx = DgramSocket<LocalSeqPacket, Tx>;
    type Rx = DgramSocket<LocalSeqPacket, Rx>;
}

impl Endpoint<LocalSeqPacket> for LocalEndpoint<LocalSeqPacket> {
    fn protocol(&self) -> LocalSeqPacket {
        LocalSeqPacket
    }

    fn as_ptr(&self) -> *const sockaddr {
        &self.sun as *const _ as *const _
    }

    fn as_mut_ptr(&mut self) -> *mut sockaddr {
        &mut self.sun as *mut _ as *mut _
    }

    fn capacity(&self) -> socklen_t {
        self.sun.capacity() as socklen_t
    }

    fn size(&self) -> socklen_t {
        self.sun.size() as socklen_t
    }

    unsafe fn resize(&mut self, size: socklen_t) {
        debug_assert!(size <= self.capacity());
        self.sun.resize(size as u8)
    }
}

impl fmt::Debug for LocalEndpoint<LocalSeqPacket> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}:{:?})", self.protocol(), self.as_pathname())
    }
}

/// The seq-packet endpoint type.
pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

pub type LocalSeqPacketBuilder = SocketBuilder<LocalSeqPacket, DgramSocket<LocalSeqPacket, Tx>, DgramSocket<LocalSeqPacket, Rx>>;

/// The seq-packet listener type.
pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket, DgramSocket<LocalSeqPacket, Tx>, DgramSocket<LocalSeqPacket, Rx>>;

/// The seq-packet socket type.
pub type LocalSeqPacketRxSocket = DgramSocket<LocalSeqPacket, Rx>;

/// The seq-packet socket type.
pub type LocalSeqPacketTxSocket = DgramSocket<LocalSeqPacket, Tx>;

#[test]
fn test_seq_packet() {
    assert!(LocalSeqPacket == LocalSeqPacket);
}

#[test]
#[cfg(target_os = "linux")]
fn test_format() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    println!("{:?}", LocalSeqPacket);
    println!("{:?}", LocalSeqPacketEndpoint::new("foo/bar").unwrap());
    println!("{:?}", LocalSeqPacketBuilder::new(ctx, LocalSeqPacket).unwrap());
}
