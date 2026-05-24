use super::{MsgBufMut, ReserveError};
use crate::socket_base::{Endpoint, EndpointRef};
use std::alloc::{Layout, LayoutError};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::{ptr, slice};

pub struct MsgBuf {
    _buf: Pin<Box<[u8]>>,
    msg: libc::msghdr,
    buf_len: usize,
    msg_len: bool,
}

impl MsgBuf {
    pub fn new(buf_len: usize) -> Result<Self, LayoutError> {
        if buf_len == 0 {
            let _ = Layout::from_size_align(0, 0)?;
        }

        // iovec: iovec
        let (layout, iovec) = (Layout::new::<libc::iovec>(), 0);
        // name: sockaddr_storage
        let (layout, name) = layout.extend(Layout::new::<libc::sockaddr_storage>())?;
        // buffer: [u8; len]
        let (layout, buffer) = layout.extend(Layout::array::<u8>(buf_len)?)?;
        // control: [u8; len]
        let (layout, control) = layout.extend(Layout::array::<u8>(1024)?)?;

        let mut buf = Box::<[u8]>::new_uninit_slice(layout.size());
        let mut msg = MaybeUninit::<libc::msghdr>::uninit();
        unsafe {
            // iovec
            let iovec: &mut libc::iovec = &mut *(buf.as_mut_ptr().add(iovec).cast());
            iovec.iov_base = buf.as_mut_ptr().add(buffer).cast();
            //iovec.iov_len = <indefinite>

            // msghdr
            let msg = &mut *msg.as_mut_ptr();
            msg.msg_iov = ptr::from_mut(iovec);
            msg.msg_iovlen = 1;
            msg.msg_name = buf.as_mut_ptr().add(name).cast();
            //msg.msg_namelen = <indefinite>;
            msg.msg_control = buf.as_mut_ptr().add(control).cast();
            msg.msg_controllen = 1024;
            msg.msg_flags = 0;
        }

        Ok(unsafe {
            Self {
                _buf: Box::into_pin(buf.assume_init()),
                msg: msg.assume_init(),
                buf_len: buf_len,
                msg_len: false,
            }
        })
    }

    pub const fn max_len(&self) -> usize {
        1
    }

    pub const fn len(&self) -> usize {
        if self.msg_len { 1 } else { 0 }
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

    pub fn as_endpoint_unchecked<E>(&self) -> EndpointRef<'_, E>
    where
        E: Endpoint,
    {
        let sa = self.msg.msg_name.cast();
        unsafe { EndpointRef::new_unchecked(&*sa, self.msg.msg_namelen) }
    }

    pub fn prepare(&mut self) -> Result<MsgBufMut<'_>, ReserveError> {
        if self.msg_len {
            Err(ReserveError)
        } else {
            Ok(MsgBufMut::new(self))
        }
    }

    pub(super) fn prepare_bytes(&self) -> &mut [u8] {
        unsafe {
            let iov = &mut *self.msg.msg_iov;
            slice::from_raw_parts_mut(iov.iov_base.cast(), self.buf_len)
        }
    }

    pub(super) fn commit<E>(&mut self, len: usize, ep: &E)
    where
        E: Endpoint,
    {
        let msg = &mut self.msg;
        let sa = msg.msg_name.cast();
        unsafe {
            *sa = *ep.sockaddr_ref();
            msg.msg_namelen = ep.sockaddr_len();
            let iov = &mut *msg.msg_iov;
            iov.iov_len = len;
        }
    }

    pub fn as_ptr(&mut self) -> *mut libc::msghdr {
        &mut self.msg
    }
}
