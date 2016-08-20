use std::cmp;
use std::mem;
use std::hash;
use std::slice;
use std::marker::PhantomData;
use {Protocol, SockAddr};
use backbone::{sockaddr, sockaddr_eq, sockaddr_cmp, sockaddr_hash};

#[derive(Clone)]
pub struct GenericEndpoint<P> {
    len: usize,
    sa: Box<[u8]>,
    marker: PhantomData<P>,
    protocol: i32,
}

impl<P> GenericEndpoint<P> {
    pub fn new<T: SockAddr>(ep: &T, protocol: i32) -> GenericEndpoint<P> {
        let mut v = vec![0; ep.capacity()];
        let len = ep.size();
        let src = unsafe { slice::from_raw_parts(ep.as_sockaddr() as *const _ as *const u8, len) };
        v[..len].copy_from_slice(src);
        GenericEndpoint {
            len: len,
            sa: v.into_boxed_slice(),
            marker: PhantomData,
            protocol: protocol,
        }
    }

    fn default(capacity: usize, protocol: i32) -> GenericEndpoint<P> {
        GenericEndpoint {
            len: 0,
            sa: vec![0; capacity].into_boxed_slice(),
            marker: PhantomData,
            protocol: protocol,
        }
    }
}

impl<P: Protocol> SockAddr for GenericEndpoint<P> {
    fn as_sockaddr(&self) -> &sockaddr {
        unsafe { mem::transmute(self.sa.as_ptr()) }
    }

    fn as_mut_sockaddr(&mut self) -> &mut sockaddr {
        unsafe { mem::transmute(self.sa.as_mut_ptr()) }
    }

    fn capacity(&self) -> usize {
        self.sa.len()
    }

    fn size(&self) -> usize {
        self.len
    }

    unsafe fn resize(&mut self, size: usize) {
        debug_assert!(size <= self.capacity());
        self.len = size;
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
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        sockaddr_cmp(self, other)
    }
}

impl<P: Protocol> PartialOrd for GenericEndpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Protocol> hash::Hash for GenericEndpoint<P> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
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
