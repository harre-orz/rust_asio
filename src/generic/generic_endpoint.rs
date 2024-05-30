use crate::socket_base::{SocklenType, Endpoint};
use libc;
use std::marker::PhantomData;
use std::fmt;
use std::mem::{self, MaybeUninit};
use crate::socket_base::SockaddrType;

#[derive(Copy, Clone)]
pub struct GenericEndpoint<P> {
    ss: libc::sockaddr_storage,
    len: SocklenType,
    _marker: PhantomData<P>,
}

impl<P> Endpoint for GenericEndpoint<P> {
    const SIZE: SocklenType = mem::size_of::<libc::sockaddr_storage>() as SocklenType;

    fn len(&self) -> SocklenType {
        self.len
    }

    fn as_ptr(&self) -> SockaddrType {
        &self.ss as *const libc::sockaddr_storage as SockaddrType
    }

    unsafe fn init(sa: MaybeUninit<Self>, len: SocklenType) -> Self {
        if len as usize >= mem::size_of::<libc::sockaddr_storage>() {
            panic!()
        }

        let mut ep = sa.assume_init();
        ep.len = len;
        ep
    }
}

impl<P> fmt::Debug for GenericEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}
