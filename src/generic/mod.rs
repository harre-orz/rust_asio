use prelude::{Protocol, SockAddr};
use ffi::{SockAddrImpl, sockaddr};

use std::slice;
use std::marker::PhantomData;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GenericEndpoint<P: Protocol> {
    sa: SockAddrImpl<Box<[u8]>>,
    protocol: i32,
    _marker: PhantomData<P>,
}

impl<P: Protocol> GenericEndpoint<P> {
    pub fn new<T>(ep: &T, protocol: i32) -> GenericEndpoint<P>
        where T: SockAddr,
    {
        let mut sa = vec![0; ep.capacity()];
        let len = ep.size();
        let src = unsafe { slice::from_raw_parts(ep.as_ref() as *const _ as *const u8, len) };
        sa[..len].copy_from_slice(src);
        GenericEndpoint {
            sa: SockAddrImpl::from_vec(sa, len),
            protocol: protocol,
            _marker: PhantomData,
        }
    }

    fn default(capacity: usize, protocol: i32) -> GenericEndpoint<P> {
        GenericEndpoint {
            sa: SockAddrImpl::from_vec(vec![0; capacity], 0),
            protocol: protocol,
            _marker: PhantomData,
        }
    }
}

impl<P: Protocol> SockAddr for GenericEndpoint<P> {
    type SockAddr = sockaddr;

    fn as_ref(&self) -> &Self::SockAddr {
        unsafe { &*(self.sa.as_ptr() as *const _) }
    }

    unsafe fn as_mut(&mut self) -> &mut Self::SockAddr {
        &mut *(self.sa.as_ptr() as *mut _)
    }

    fn capacity(&self) -> usize {
        self.sa.capacity()
    }

    fn size(&self) -> usize {
        self.sa.size()
    }

    unsafe fn resize(&mut self, size: usize) {
        debug_assert!(size <= self.capacity());
        self.sa.resize(size)
    }
}

mod stream;
pub use self::stream::*;

mod dgram;
pub use self::dgram::*;

mod raw;
pub use self::raw::*;

mod seq_packet;
pub use self::seq_packet::*;
