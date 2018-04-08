use ffi::{sockaddr, socklen_t, AF_UNIX, SOCK_DGRAM};
use core::{Endpoint, Protocol};
use dgram_socket::DgramSocket;
use local::LocalEndpoint;

use std::fmt;
use std::mem;

/// The datagram-oriented UNIX domain protocol.
///
/// # Example
/// Create a server and client sockets.
///
/// ```rust,no_run
/// use asyncio::{IoContext, Endpoint};
/// use asyncio::local::{LocalDgram, LocalDgramEndpoint, LocalDgramSocket};
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = LocalDgramEndpoint::new("example.sock").unwrap();
///
/// let sv = LocalDgramSocket::new(ctx, LocalDgram).unwrap();
/// sv.bind(&ep).unwrap();
///
/// let cl = LocalDgramSocket::new(ctx, ep.protocol()).unwrap();
/// cl.connect(&ep).unwrap();
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct LocalDgram;

impl Protocol for LocalDgram {
    type Endpoint = LocalEndpoint<Self>;

    type Socket = LocalDgramSocket;

    fn family_type(&self) -> i32 {
        AF_UNIX
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl Endpoint<LocalDgram> for LocalEndpoint<LocalDgram> {
    fn protocol(&self) -> LocalDgram {
        LocalDgram
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

impl fmt::Debug for LocalEndpoint<LocalDgram> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}:{:?})", self.protocol(), self.as_pathname())
    }
}

/// The datagram-oriented UNIX domain endpoint type.
pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

/// The datagram-oriented UNIX domain socket type.
pub type LocalDgramSocket = DgramSocket<LocalDgram>;

// #[test]
// fn test_format() {
//     use core::IoContext;
//
//     let ctx = &IoContext::new().unwrap();
//     println!("{:?}", LocalDgram);
//     println!("{:?}", LocalDgramEndpoint::new("foo/bar").unwrap());
//     println!("{:?}", LocalDgramSocket::new(ctx, LocalDgram).unwrap());
// }
