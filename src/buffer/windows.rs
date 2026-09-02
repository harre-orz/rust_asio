use crate::socket_base::{Endpoint, EndpointRef};
use std::alloc::{Layout, LayoutError};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::{ptr, slice};
use windows_sys::Win32::Networking::WinSock;

pub struct MsgBuf {
    _buf: Pin<Box<[u8]>>,
    msg: WinSock::WSAMSG,
    buf_len: usize,
}

impl MsgBuf {
    pub fn as_msghdr(&mut self) -> *mut WinSock::WSAMSG {
        ptr::from_mut(&mut self.msg)
    }

    pub fn as_bytes(&self) -> &[u8] {
        if self.msg_len {
            unsafe {
                let iov = &*self.msg.msg_iov;
                slice::from_raw_parts(iov.iov_base.cast(), self.buf_len)
            }
        } else {
            &[]
        }
    }
}

//     pub const fn len(&self) -> usize {
//         if self.msg_len { 1 } else { 0 }
//     }
//

//

//
//     pub fn prepare(&mut self) -> Result<MsgBufMut<'_>, TryReserveError> {
//         if self.msg_len {
//             Err(TryReserveError)
//         } else {
//             Ok(MsgBufMut(self))
//         }
//     }
//
//     pub(super) fn prepare_bytes(&self) -> &mut [u8] {
//         &mut []
//     }
//
//     pub(super) fn commit<E>(&mut self, len: usize, ep: &E)
//     where
//         E: Endpoint,
//     {
//     }
//

// }
