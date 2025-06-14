use crate::sockaddr::ffi::SockAddrStorage;
use crate::socket_base::{AddressFamily, Endpoint};
use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub struct GenericEndpoint<P> {
    inner: SockAddrStorage,
    _marker: PhantomData<P>,
}

impl<P> GenericEndpoint<P> {
    pub fn new(family_type: AddressFamily, bytes: &[u8]) -> Option<Self> {
        SockAddrStorage::new(family_type, bytes).map(|ss| Self {
            inner: ss,
            _marker: PhantomData,
        })
    }

    pub const fn family_type(&self) -> AddressFamily {
        self.inner.family_type()
    }

    pub const fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

impl<P> Endpoint for GenericEndpoint<P> {
    type SockAddr = SockAddrStorage;

    fn new(sa: Self::SockAddr) -> Self {
        Self {
            inner: sa,
            _marker: PhantomData,
        }
    }

    fn sockaddr(&self) -> &Self::SockAddr {
        &self.inner
    }
}
