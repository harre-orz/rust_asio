use std::alloc::{Layout, LayoutError};
use std::pin::Pin;
use std::{mem, ptr, slice};
use std::marker::PhantomData;
use crate::sockaddr::SockLen;
use crate::socket_base::{Endpoint, EndpointRef};

pub struct MsgBuf<E>
where
    E: Endpoint
{
    _bufs: Box<[Pin<Box<[u8]>>]>,
    msgs: Box<[libc::mmsghdr]>,
    buf_len: usize,
    wpos: usize,
    rpos: usize,
    _marker: PhantomData<E>,
}

impl<E> MsgBuf<E>
where
    E: Endpoint
{
    const NAME_LEN: SockLen = size_of::<libc::sockaddr_storage>() as SockLen;
    const CONTROL_LEN: usize = 1024;

    pub fn new(len: usize) -> Result<Self, LayoutError> {
        Self::with_quantity(len, 1)
    }

    pub fn with_quantity(len: usize, quantity: usize) -> Result<Self, LayoutError> {
        if len == 0 || quantity == 0 {
            let _ = Layout::from_size_align(0, 0)?;
        }

        // iovec: iovec
        let (layout, iovec) = (Layout::new::<libc::iovec>(), 0);
        // name: sockaddr_storage
        let (layout, name) = layout.extend(Layout::new::<libc::sockaddr_storage>())?;
        // control: [u8; 1024]
        let (layout, control) = layout.extend(Layout::array::<u8>(1024)?)?;
        // buffer: [u8; buf_len]
        let (layout, buffer) = layout.extend(Layout::array::<u8>(len)?)?;

        let mut bufs = Vec::new();
        let mut msgs = Box::<[libc::mmsghdr]>::new_uninit_slice(quantity);
        for i in 0..quantity {
            let mut buf = Box::<[u8]>::new_uninit_slice(layout.size());
            unsafe {
                // iovec
                let iov: &mut libc::iovec = &mut *(buf.as_mut_ptr().add(iovec).cast());
                iov.iov_base = buf.as_mut_ptr().add(buffer).cast();
                //iov.iov_len = ...

                // msghdr
                let msg: &mut libc::mmsghdr = &mut *msgs[i].as_mut_ptr();
                msg.msg_hdr.msg_iov = ptr::from_mut(iov);
                msg.msg_hdr.msg_iovlen = 1;
                msg.msg_hdr.msg_name = buf.as_mut_ptr().add(name).cast();
                //msg.msg_hdr.msg_namelen = ...;
                msg.msg_hdr.msg_control = buf.as_mut_ptr().add(control).cast();
                //msg.msg_hdr.msg_controllen = Self::CONTROL_LEN;
                msg.msg_hdr.msg_flags = 0;
                bufs.push(Box::into_pin(buf.assume_init()))
            }
        }
        let mut msgs = unsafe { msgs.assume_init() };
        let msg = &mut msgs[0].msg_hdr;
        msg.msg_namelen = Self::NAME_LEN;
        msg.msg_controllen = Self::CONTROL_LEN;
        Ok(Self {
            _bufs: Vec::into_boxed_slice(bufs),
            msgs: msgs,
            buf_len: len,
            rpos: 0,
            wpos: 0,
            _marker: PhantomData,
        })
    }

    pub(crate) const fn mmsghdr_recv_next(&mut self) -> Result<usize, &mut [libc::mmsghdr]> {
        if self.rpos + 1 < self.wpos {
            let msg = &mut self.msgs[self.rpos].msg_hdr;
            msg.msg_namelen = Self::NAME_LEN;
            //msg.msg_controllen = Self::CONTROL_LEN;
            unsafe { &mut *msg.msg_iov }.iov_len = self.buf_len;
            self.rpos += 1;
            Ok(self.msgs[self.rpos].msg_len as usize)
        } else {
            Err(&mut self.msgs)
        }
    }

    pub(crate) unsafe fn mmsghdr_set_len(&mut self, len: usize) -> usize {
        self.wpos = len;
        self.rpos = 0;
        self.msgs[0].msg_len as usize
    }

    pub const fn len(&self) -> usize {
        self.buf_len
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let iov = &mut *self.msgs[self.rpos].msg_hdr.msg_iov;
            slice::from_raw_parts(iov.iov_base.cast(), self.buf_len)
        }
    }

    pub fn prepare(&mut self) -> MsgBufMut<'_>
    {
        let msg = &self.msgs[self.rpos].msg_hdr;
        let iov = unsafe { &mut *msg.msg_iov };
        iov.iov_len = self.buf_len;
        MsgBufMut { iov: iov }
    }
}

pub struct MsgBufMut<'a> {
    iov: &'a mut libc::iovec,
}

impl<'a> MsgBufMut<'a> {
    pub const fn len(&self) -> usize {
       self.iov.iov_len
    }

    pub const fn as_bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self.iov.iov_base as *const u8, self.iov.iov_len)
        }
    }

    pub const fn as_bytes_mut(&self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.iov.iov_base as *mut u8, self.iov.iov_len)
        }
    }

    pub const fn commit(self, len: usize) {
        self.iov.iov_len = len;
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr};
    use crate::ip::{UdpEndpoint};
    use crate::socket_base::Endpoint;
    use super::*;

    impl MsgBuf<UdpEndpoint> {
        fn pseudo_recvmsg<const N: usize>(&mut self, data: [(&[u8], UdpEndpoint); N]) -> usize
        {
            unsafe {
                for (i, (buf, ep)) in data.iter().enumerate() {
                    let mmsg = &mut self.msgs[i];
                    ptr::copy_nonoverlapping::<u8>(ptr::from_ref(ep.sockaddr_ref()).cast(), mmsg.msg_hdr.msg_name.cast(), ep.sockaddr_len() as usize);
                    let iov = &mut *mmsg.msg_hdr.msg_iov;
                    ptr::copy_nonoverlapping::<u8>(buf.as_ptr(), iov.iov_base.cast(), buf.len());
                    mmsg.msg_len = buf.len() as libc::c_uint;
                }
                self.mmsghdr_set_len(data.len())
            }
        }
    }

    #[test]
    fn test_msgbuf() {
        let mbuf: MsgBuf<UdpEndpoint> = MsgBuf::new(1024).unwrap();
        assert_eq!(mbuf.len(), 1024);
        assert_eq!(mbuf.as_bytes().len(), 1024);
    }

    #[test]
    fn test_msgbuf_read_1() {
        let mut mbuf = MsgBuf::new(1024).unwrap();
        let len= mbuf.pseudo_recvmsg([
            ("hello".as_bytes(), UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 1)),
        ]);
        assert_eq!(len, 5);
        assert_eq!(&mbuf.as_bytes()[..len], "hello".as_bytes());
        assert!(mbuf.mmsghdr_recv_next().is_err());
    }

    #[test]
    fn test_msgbuf_read_2() {
        let mut mbuf = MsgBuf::with_quantity(1024, 5).unwrap();
        let len= mbuf.pseudo_recvmsg([
            ("hello".as_bytes(), UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 1)),
            ("world".as_bytes(), UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 2)),
        ]);
        assert_eq!(len, 5);
        assert_eq!(&mbuf.as_bytes()[..len], "hello".as_bytes());

        let len = mbuf.mmsghdr_recv_next().unwrap();
        assert_eq!(len, 5);
        assert_eq!(&mbuf.as_bytes()[..len], "world".as_bytes());
        assert!(mbuf.mmsghdr_recv_next().is_err());
    }
}
