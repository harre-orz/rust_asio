use super::{MsgBufMut, ReserveError};
use crate::sockaddr::SockLen;
use crate::socket_base::{Endpoint, EndpointRef};
use std::alloc::{Layout, LayoutError};
use std::pin::Pin;
use std::{ptr, slice};

pub struct MsgBuf {
    _bufs: Box<[Pin<Box<[u8]>>]>,
    msgs: Box<[libc::mmsghdr]>,
    buf_len: usize,
    wpos: usize,
    rpos: usize,
}

impl MsgBuf {
    pub fn new(buf_len: usize) -> Result<Self, LayoutError> {
        Self::with_max_len(buf_len, 1)
    }

    pub fn with_max_len(buf_len: usize, msg_len: usize) -> Result<Self, LayoutError> {
        if buf_len == 0 || msg_len == 0 {
            let _ = Layout::from_size_align(0, 0)?;
        }

        // iovec: iovec
        let (layout, iovec) = (Layout::new::<libc::iovec>(), 0);
        // name: sockaddr_storage
        let (layout, name) = layout.extend(Layout::new::<libc::sockaddr_storage>())?;
        // buffer: [u8; buf_len]
        let (layout, buffer) = layout.extend(Layout::array::<u8>(buf_len)?)?;
        // control: [u8; 1024]
        let (layout, control) = layout.extend(Layout::array::<u8>(1024)?)?;

        let mut bufs = Vec::new();
        let mut msgs = Box::<[libc::mmsghdr]>::new_uninit_slice(msg_len);
        for i in 0..msgs.len() {
            let mut buf = Box::<[u8]>::new_uninit_slice(layout.size());
            unsafe {
                // iovec
                let iov: &mut libc::iovec = &mut *(buf.as_mut_ptr().add(iovec).cast());
                iov.iov_base = buf.as_mut_ptr().add(buffer).cast();
                //iov.iov_len = <indefinite>

                // msghdr
                let msg: &mut libc::mmsghdr = &mut *msgs[i].as_mut_ptr();
                msg.msg_hdr.msg_iov = ptr::from_mut(iov);
                msg.msg_hdr.msg_iovlen = 1;
                msg.msg_hdr.msg_name = buf.as_mut_ptr().add(name).cast();
                //msg.msg_hdr.msg_namelen = <indefinite>;
                msg.msg_hdr.msg_control = buf.as_mut_ptr().add(control).cast();
                msg.msg_hdr.msg_controllen = 1024;
                msg.msg_hdr.msg_flags = 0;
                bufs.push(Box::into_pin(buf.assume_init()))
            }
        }
        Ok(Self {
            _bufs: Vec::into_boxed_slice(bufs),
            msgs: unsafe { msgs.assume_init() },
            buf_len: buf_len,
            rpos: 0,
            wpos: 0,
        })
    }

    pub(crate) fn next(&mut self) -> Option<usize> {
        if self.rpos < self.wpos {
            let len = self.msgs[self.rpos].msg_len as usize;
            self.rpos += 1;
            Some(len)
        } else {
            None
        }
    }

    pub(crate) fn as_mut_slice(&mut self) -> &mut [libc::mmsghdr] {
        &mut self.msgs
    }

    pub(crate) unsafe fn uninit(&mut self) {
        for msg in &mut self.msgs {
            msg.msg_hdr.msg_namelen = size_of::<libc::sockaddr_storage>() as SockLen;
            let iov = unsafe { &mut *msg.msg_hdr.msg_iov };
            iov.iov_len = self.buf_len;
        }
    }

    pub(crate) unsafe fn set_len(&mut self, len: usize) -> usize {
        self.rpos = 1;
        self.wpos = len;
        self.msgs[0].msg_len as usize
    }

    pub const fn max_len(&self) -> usize {
        self.msgs.len()
    }

    pub const fn len(&self) -> usize {
        self.wpos - self.rpos
    }

    pub fn as_bytes(&self) -> &[u8] {
        if self.rpos > 0 {
            unsafe {
                let iov = &mut *self.msgs[self.rpos - 1].msg_hdr.msg_iov;
                slice::from_raw_parts(iov.iov_base.cast(), self.buf_len)
            }
        } else {
            &mut []
        }
    }

    pub unsafe fn as_endpoint_unchecked<E>(&self) -> EndpointRef<'_, E>
    where
        E: Endpoint,
    {
        let len = if self.rpos > 0 { self.rpos - 1 } else { 0 };
        let msg = &self.msgs[len];
        let sa = msg.msg_hdr.msg_name.cast();
        unsafe { EndpointRef::new_unchecked(&*sa, msg.msg_hdr.msg_namelen) }
    }

    pub fn prepare(&mut self) -> Result<MsgBufMut<'_>, ReserveError> {
        if self.wpos < self.msgs.len() {
            Ok(MsgBufMut::new(self))
        } else {
            Err(ReserveError)
        }
    }

    pub(super) fn prepare_bytes(&self) -> &mut [u8] {
        unsafe {
            let iov = &mut *self.msgs[self.wpos].msg_hdr.msg_iov;
            slice::from_raw_parts_mut(iov.iov_base.cast(), self.buf_len)
        }
    }

    pub(super) fn commit<E>(&mut self, len: usize, ep: &E)
    where
        E: Endpoint,
    {
        let msg = &mut self.msgs[self.wpos];
        let sa = msg.msg_hdr.msg_name.cast();
        unsafe {
            *sa = *ep.sockaddr_ref();
            msg.msg_hdr.msg_namelen = ep.sockaddr_len();
            let iov = &mut *msg.msg_hdr.msg_iov;
            iov.iov_len = len;
        }
        msg.msg_len = len as _;
        self.wpos += 1;
    }
}

#[test]
fn test_msgbuf() {
    let mbuf = MsgBuf::new(1024).unwrap();
    assert_eq!(mbuf.len(), 0);
    assert_eq!(mbuf.max_len(), 1);
    assert_eq!(mbuf.as_bytes(), &[]);
}

#[test]
fn test_msgbuf_as_endpoint_unchecked() {
    use crate::ip::UdpEndpoint;

    let mbuf = MsgBuf::new(1024).unwrap();
    assert_eq!(mbuf.len(), 0);
    assert_eq!(mbuf.max_len(), 1);
    unsafe {
        // returns indefinite, but overflow
        mbuf.as_endpoint_unchecked::<UdpEndpoint>();
    }
}

#[test]
fn test_msgbuf_prepare() {
    use crate::ip::UdpEndpoint;
    use std::net::Ipv4Addr;

    let ep = UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345);
    let mut mbuf = MsgBuf::new(1024).unwrap();

    let buf = mbuf.prepare().unwrap();
    assert_eq!(buf.len(), 1024);

    buf.commit(100, &ep);
    assert_eq!(mbuf.len(), 1);
    assert_eq!(mbuf.max_len(), 1);
    assert_eq!(mbuf.as_bytes().len(), 0);
}

#[test]
fn test_msgbuf_prepare_next() {
    use crate::ip::UdpEndpoint;
    use std::io::Write;
    use std::net::Ipv4Addr;

    let ep = UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345);
    let mut mbuf = MsgBuf::new(1024).unwrap();

    let mut buf = mbuf.prepare().unwrap();
    let len = buf.as_bytes_mut().write(b"hello world").unwrap();
    buf.commit(len, &ep);
    assert_eq!(mbuf.as_bytes(), &[]);
    assert_eq!(mbuf.next().unwrap(), len);
    assert_eq!(&mbuf.as_bytes()[..len], b"hello world");
    assert_eq!(&unsafe { mbuf.as_endpoint_unchecked() }, &ep);
}

#[test]
fn test_msgbuf_prepare_commit() {
    let mbuf = MsgBuf::with_max_len(1024, 3).unwrap();
    assert_eq!(mbuf.len(), 0);
    assert_eq!(mbuf.max_len(), 3);
}

#[test]
fn test_msgbuf_3_prepare() {
    let mbuf = MsgBuf::with_max_len(1024, 3).unwrap();
    assert_eq!(mbuf.len(), 0);
    assert_eq!(mbuf.max_len(), 3);
}
