use crate::buffer::ReserveError;
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
    msg_len: bool,
}

impl MsgBuf {
    pub const fn max_len(&self) -> usize {
        1
    }

    pub const fn len(&self) -> usize {
        if self.msg_len { 1 } else { 0 }
    }

    // pub fn as_bytes(&self) -> &[u8] {
    //     if self.msg_len {
    //         unsafe {
    //             let iov = &*self.msg.msg_iov;
    //             slice::from_raw_parts(iov.iov_base.cast(), self.buf_len)
    //         }
    //     } else {
    //         &[]
    //     }
    // }

    // pub fn as_endpoint_unchecked<E>(&self) -> EndpointRef<'_, E>
    // where
    //     E: Endpoint,
    // {
    //     let sa = self.msg.msg_name.cast();
    //     unsafe { EndpointRef::new_unchecked(&*sa, self.msg.msg_namelen) }
    // }

    pub fn prepare(&mut self) -> Result<MsgBufMut<'_>, ReserveError> {
        if self.msg_len {
            Err(ReserveError)
        } else {
            Ok(MsgBufMut(self))
        }
    }

    pub(super) fn prepare_bytes(&self) -> &mut [u8] {
        &mut []
    }

    pub(super) fn commit<E>(&mut self, len: usize, ep: &E)
    where
        E: Endpoint,
    {
    }

    pub fn as_ptr(&self) -> *const WinSock::WSAMSG {
        ptr::from_ref(&self.msg) as *const WinSock::WSAMSG
    }
}

pub struct MsgBufMut<'a>(&'a mut MsgBuf);
