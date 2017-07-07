use ffi::{SockAddr};
use prelude::{Endpoint, Protocol};

use std::slice;
use std::marker::{PhantomData};
use libc::{sockaddr, socklen_t};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GenericEndpoint<P> {
    sa: SockAddr<Box<[u8]>>,
    protocol: i32,
    _marker: PhantomData<P>,
}

impl<P> GenericEndpoint<P> {
    pub fn new(ep: Vec<u8>, protocol: i32) -> GenericEndpoint<P> {
        let mut sa = vec![0; ep.capacity()];
        let len = ep.len();
        let src = unsafe { slice::from_raw_parts(ep.as_ptr() as *const _ as *const u8, len) };
        sa[..len].copy_from_slice(src);
        GenericEndpoint {
            sa: SockAddr::from_vec(sa, len as u8),
            protocol: protocol,
            _marker: PhantomData,
        }
    }

    fn default(capacity: socklen_t, protocol: i32) -> GenericEndpoint<P> {
        GenericEndpoint {
            sa: SockAddr::from_vec(vec![0; capacity as usize], 0),
            protocol: protocol,
            _marker: PhantomData,
        }
    }
}

impl<P: Protocol> Endpoint for GenericEndpoint<P> {
    fn as_ptr(&self) -> *const sockaddr {
        self.sa.sa.as_ptr() as *const _
    }

    fn as_mut_ptr(&mut self) -> *mut sockaddr {
        self.sa.sa.as_mut_ptr() as *mut _
    }

    fn capacity(&self) -> socklen_t {
        self.sa.capacity() as socklen_t
    }

    fn size(&self) -> socklen_t {
        self.sa.size() as socklen_t
    }

    unsafe fn resize(&mut self, size: socklen_t) {
        debug_assert!(size <= self.capacity());
        self.sa.resize(size as u8)
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
