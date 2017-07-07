use ffi::*;
use prelude::{IoControl, SocketOption, GetSocketOption, SetSocketOption};

use libc::c_void;

pub const MAX_CONNECTIONS: i32 = 126;

pub struct Tx;

pub struct Rx;

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

    fn as_mut_ptr(&mut self) -> *mut c_void {
        &mut self.0 as *mut _ as *mut _
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
    fn name(&self) -> u64 { FIONREAD as u64 }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        &mut self.0 as *mut _ as *mut _
    }
}
