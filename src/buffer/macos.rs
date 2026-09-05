use crate::socket_base::{Endpoint, EndpointRef};
use std::alloc::{Layout, LayoutError};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::{ptr, slice};
use libc::socklen_t;

pub struct MsgBuf {
    _buf: Pin<Box<[u8]>>,
    buf_len: usize,
    msg: libc::msghdr,
}

impl MsgBuf {
    const CONTROL_LEN: socklen_t = 1024;

    pub fn new(buf_len: usize) -> Result<Self, LayoutError> {
        if buf_len == 0 {
            let _ = Layout::from_size_align(0, 0)?;
        }

        // iovec: iovec
        let (layout, iovec) = (Layout::new::<libc::iovec>(), 0);
        // name: sockaddr_storage
        let (layout, name) = layout.extend(Layout::new::<libc::sockaddr_storage>())?;
        // control: [u8; len]
        let (layout, control) = layout.extend(Layout::array::<u8>(1024)?)?;
        // buffer: [u8; len]
        let (layout, buffer) = layout.extend(Layout::array::<u8>(buf_len)?)?;

        let mut buf = Box::<[u8]>::new_uninit_slice(layout.size());
        let mut msg = MaybeUninit::<libc::msghdr>::uninit();
        unsafe {
            // iovec
            let iovec: &mut libc::iovec = &mut *(buf.as_mut_ptr().add(iovec).cast());
            iovec.iov_base = buf.as_mut_ptr().add(buffer).cast();
            //iovec.iov_len = <indefinite>;

            // msghdr
            let msg = &mut *msg.as_mut_ptr();
            msg.msg_iov = ptr::from_mut(iovec);
            msg.msg_iovlen = 1;
            msg.msg_name = buf.as_mut_ptr().add(name).cast();
            msg.msg_namelen = 0;
            msg.msg_control = buf.as_mut_ptr().add(control).cast();
            msg.msg_controllen = Self::CONTROL_LEN;
            msg.msg_flags = 0;
        }

        Ok(unsafe {
            Self {
                _buf: Box::into_pin(buf.assume_init()),
                buf_len: buf_len,
                msg: msg.assume_init(),
            }
        })
    }

    pub(crate) const fn as_msghdr(&mut self) -> *mut libc::msghdr {
        &mut self.msg
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let base: *const u8 = (*self.msg.msg_iov).iov_base.cast();
            slice::from_raw_parts(base, self.len)
        }
    }

    pub fn as_endpoint<E>(&self) -> Option<EndpointRef<E>>
    where
        E: Endpoint
    {
        if self.msg.msg_namelen > 0 {
            let sa = self.msg.msg_name as *const E::SockAddr;
            Some(unsafe { EndpointRef::new_unchecked(&*sa, self.msg.msg_namelen) })
        } else {
            None
        }
    }

    pub fn prepare(&mut self) -> MsgBufMut {
        MsgBufMut { buf: self }
    }
}

pub struct MsgBufMut<'a> {
    iov: &'a mut libc::iovec,
}

impl<'a> MsgBufMut<'a> {
    pub fn len(&self) -> usize {
        self.buf.len
    }

    pub fn as_ptr(&self) -> *mut u8 {
        unsafe {
            (*self.buf.msg.msg_iov).iov_base as *mut u8
        }
    }

    pub fn as_slice(&self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.as_ptr(), self.buf.len)
        }
    }

    pub fn endpoint<E>(&self, endpoint: &E)
    where
        E: Endpoint
    {
        let src: *const u8 = ptr::from_ref(endpoint.sockaddr_ref()).cast();
        let dst: *mut u8 = self.buf.msg.msg_name.cast();
        unsafe { ptr::copy_nonoverlapping(src, dst, endpoint.sockaddr_len() as usize); }
        //self.buf.msg.msg_namelen = endpoint.sockaddr_len();
    }

    pub fn commit(self, len: usize)
    {
        unsafe { &mut *self.buf.msg.msg_iov }.iov_len = len;
    }
}

#[test]
fn test_msgbuf_new() {
    let mbuf = MsgBuf::new(1024).unwrap();

}