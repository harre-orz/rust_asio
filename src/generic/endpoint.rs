use crate::sockaddr::{AddressFamily, SockAddrStorage, SockAddrWithLen, SockLen};
use crate::socket_base::{Endpoint, EndpointIntoIter, EndpointIter, Endpoints, Protocol};
use std::fmt;
use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub struct GenericEndpoint<P>
where
    P: Protocol<Endpoint = Self>,
{
    ss: SockAddrStorage,
    #[cfg(not(target_os = "macos"))]
    ss_len: SockLen,
    _marker: PhantomData<P>,
}

impl<P> GenericEndpoint<P>
where
    P: Protocol<Endpoint = Self>,
{
    pub fn new(family_type: AddressFamily, bytes: &[u8]) -> Option<Self> {
        SockAddrStorage::new(family_type, bytes).map(|ss| {
            let (ss, _ss_len) = ss.unwrap();
            Self {
                ss: ss,
                #[cfg(not(target_os = "macos"))]
                ss_len: _ss_len,
                _marker: PhantomData,
            }
        })
    }

    #[cfg(not(target_os = "macos"))]
    pub const fn len(&self) -> SockLen {
        self.ss_len as SockLen
    }
    #[cfg(target_os = "macos")]
    pub const fn len(&self) -> SockLen {
        self.ss.len() as SockLen
    }

    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { self.ss.as_bytes_unchecked(self.len()) }
    }
}

impl<P> fmt::Debug for GenericEndpoint<P>
where
    P: Protocol<Endpoint = Self>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GenericEndpoint {{ {:?} }}", self.as_bytes())
    }
}

impl<P> Endpoint for GenericEndpoint<P>
where
    P: Protocol<Endpoint = Self>,
{
    type SockAddr = SockAddrStorage;

    fn sockaddr_ref(&self) -> &Self::SockAddr {
        &self.ss
    }

    fn sockaddr_len(&self) -> SockLen {
        self.len() as SockLen
    }

    unsafe fn from_sockaddr(sa_with_len: SockAddrWithLen<Self::SockAddr>) -> Self {
        let (ss, _ss_len) = sa_with_len.unwrap();
        Self {
            ss: ss,
            #[cfg(not(target_os = "macos"))]
            ss_len: _ss_len,
            _marker: PhantomData,
        }
    }
}

impl<'a, P> Endpoints<'a, P> for &'a GenericEndpoint<P>
where
    P: Protocol<Endpoint = GenericEndpoint<P>> + 'a,
{
    type Iter = EndpointIter<'a, P>;

    fn endpoints(self) -> Self::Iter {
        EndpointIter::new(self)
    }
}

impl<'a, P> Endpoints<'a, P> for GenericEndpoint<P>
where
    P: Protocol<Endpoint = Self> + 'a,
{
    type Iter = EndpointIntoIter<'a, P>;

    fn endpoints(self) -> Self::Iter {
        EndpointIntoIter::new(self)
    }
}
