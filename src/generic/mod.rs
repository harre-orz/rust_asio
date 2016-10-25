use std::slice;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use libc::sockaddr;
use traits::{Protocol, SockAddr};
use sa_ops::{SockAddrImpl, sockaddr_eq, sockaddr_cmp, sockaddr_hash};

#[derive(Clone)]
pub struct GenericEndpoint<P: Protocol> {
    sa: SockAddrImpl<Box<[u8]>>,
    protocol: i32,
    _marker: PhantomData<P>,
}

impl<P: Protocol> GenericEndpoint<P> {
    pub fn new<T>(ep: &T, protocol: i32) -> GenericEndpoint<P>
        where T: SockAddr,
    {
        let mut v = vec![0; ep.capacity()];
        let len = ep.size();
        let src = unsafe { slice::from_raw_parts(ep.as_sockaddr() as *const _ as *const u8, len) };
        v[..len].copy_from_slice(src);
        GenericEndpoint {
            sa: SockAddrImpl::from_vec(v, len),
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
    fn as_sockaddr(&self) -> &sockaddr {
        unsafe { self.sa.as_sockaddr() }
    }

    fn as_mut_sockaddr(&mut self) -> &mut sockaddr {
        unsafe { self.sa.as_mut_sockaddr() }
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

impl<P: Protocol> Eq for GenericEndpoint<P> {
}

impl<P: Protocol> PartialEq for GenericEndpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        sockaddr_eq(self, other)
    }
}

impl<P: Protocol> Ord for GenericEndpoint<P> {
    fn cmp(&self, other: &Self) -> Ordering {
        sockaddr_cmp(self, other)
    }
}

impl<P: Protocol> PartialOrd for GenericEndpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Protocol> Hash for GenericEndpoint<P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        sockaddr_hash(self, state)
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
