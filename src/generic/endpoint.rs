//

use libc;
use socket_base::Endpoint;
use std::marker::PhantomData;

pub struct GenericEndpoint<P> {
    sa: Box<[u8]>,
    len: libc::socklen_t,
    _marker: PhantomData<P>,
}

impl<P> GenericEndpoint<P> {
    pub fn new(sa: Box<[u8]>) -> GenericEndpoint<P> {
        let len = sa.len() as _;
        GenericEndpoint {
            sa: sa,
            len: len,
            _marker: PhantomData,
        }
    }
}

impl<P> Endpoint<P> for GenericEndpoint<P> {
    fn as_ptr(&self) -> *const libc::sockaddr {
        &*self.sa as *const _ as *const _
    }

    fn as_mut_ptr(&mut self) -> *mut libc::sockaddr {
        &mut *self.sa as *mut _ as *mut _
    }

    fn capacity(&self) -> libc::socklen_t {
        self.sa.len() as _
    }

    fn size(&self) -> libc::socklen_t {
        self.len
    }

    unsafe fn resize(&mut self, len: libc::socklen_t) {
        assert!(len <= self.capacity());
        self.len = len
    }
}
