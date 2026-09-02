use crate::socket_base::{Endpoint, EndpointRef};
use std::alloc::{Layout, LayoutError};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::{ptr, slice};

pub struct MsgBuf {
    _buf: Pin<Box<[u8]>>,
    msg: libc::msghdr,
    buf_len: usize,
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
            //iovec.iov_len = <indefinite>;

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
            }
        })
    }

    pub const fn as_msghdr(&mut self) -> *mut libc::msghdr {
        &mut self.msg
    }
}
