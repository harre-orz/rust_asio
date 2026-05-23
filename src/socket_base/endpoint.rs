use crate::sockaddr::{AddressFamily, SockAddr, SockAddrWithLen, SockLen};
use crate::socket::SocketType;
use std::marker::PhantomData;
use std::{ptr, slice};

/// An abstract type of the source or destination point.
pub trait Endpoint {
    type SockAddr: SockAddr;

    fn sockaddr_ref(&self) -> &Self::SockAddr;
    fn sockaddr_len(&self) -> SockLen;

    unsafe fn from_sockaddr(sa_with_len: SockAddrWithLen<Self::SockAddr>) -> Self;
}

/// The abstract reference type of `*Endpoint`.
pub struct EndpointRef<'a, E>
where
    E: Endpoint,
{
    sa_ref: &'a E::SockAddr,
    sa_len: SockLen,
}

impl<'a, E> EndpointRef<'a, E>
where
    E: Endpoint,
{
    /// Creates from `*Endpoint`.
    pub fn new(ep: &'a E) -> Self {
        EndpointRef {
            sa_ref: ep.sockaddr_ref(),
            sa_len: ep.sockaddr_len(),
        }
    }

    /// Creates from `Sockaddr` and `SockLen`.
    pub unsafe fn new_unchecked(sa_ref: &'a E::SockAddr, sa_len: SockLen) -> Self {
        assert!(sa_len as usize <= size_of::<E::SockAddr>());
        Self { sa_ref, sa_len }
    }

    /// Returns bytes of `SockAddr`
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self.sa_ref).cast(), self.sa_len as usize) }
    }

    /// Returns `SockAddr` type.
    pub fn sockaddr_ref(&self) -> &'a E::SockAddr {
        self.sa_ref
    }

    /// Returns size of `SockAddr`.
    pub fn sockaddr_len(&self) -> SockLen {
        self.sa_len
    }
}

impl<'a, E> EndpointRef<'a, E>
where
    E: Endpoint,
{
    /// Returns owned `Endpoint`.
    pub fn clone(&self) -> E {
        let sa = *self.sa_ref;
        unsafe { E::from_sockaddr(SockAddrWithLen::new_unchecked(sa, self.sa_len)) }
    }
}

impl<'a, E> PartialEq<E> for EndpointRef<'a, E>
where
    E: Endpoint,
{
    fn eq(&self, other: &E) -> bool {
        self.as_bytes() == EndpointRef::new(other).as_bytes()
    }
}

impl<'a, E> PartialEq for EndpointRef<'a, E>
where
    E: Endpoint,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'a, E> Eq for EndpointRef<'a, E> where E: Endpoint {}

/// An abstract socket protocol type.
pub trait Protocol: Copy {
    type Endpoint: Endpoint;
    type Type: Copy + Into<i32>;

    fn new(ep: &EndpointRef<Self::Endpoint>, protocol: Self::Type) -> Self;
    fn family_type(self) -> AddressFamily;
    fn socket_type(self) -> SocketType;
    fn protocol_type(self) -> Self::Type;
}

/// An abstract iteration of the source or destination points.
pub trait Endpoints<'a, P>
where
    P: Protocol + 'a,
{
    type Iter: Iterator<Item = EndpointRef<'a, P::Endpoint>>;

    fn endpoints(self) -> Self::Iter;
}

/// An iteration type of `*Endpoint`.
pub struct EndpointIter<'a, P>(Option<&'a P::Endpoint>)
where
    P: Protocol;

impl<'a, P> EndpointIter<'a, P>
where
    P: Protocol,
{
    pub(crate) fn new(ep: &'a P::Endpoint) -> Self {
        EndpointIter(Some(ep))
    }
}

impl<'a, P> Iterator for EndpointIter<'a, P>
where
    P: Protocol,
{
    type Item = EndpointRef<'a, P::Endpoint>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().map(|ep| EndpointRef::new(ep))
    }
}

pub struct EndpointIntoIter<'a, P>(P::Endpoint, Option<PhantomData<&'a ()>>)
where
    P: Protocol;

impl<'a, P> EndpointIntoIter<'a, P>
where
    P: Protocol,
{
    pub(crate) fn new(ep: P::Endpoint) -> Self {
        Self(ep, Some(PhantomData))
    }
}

impl<'a, P> Iterator for EndpointIntoIter<'a, P>
where
    P: Protocol + 'a,
{
    type Item = EndpointRef<'a, P::Endpoint>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(_) = self.1.take() {
            let ep = &self.0 as *const P::Endpoint;
            Some(EndpointRef::new(unsafe { &*ep }))
        } else {
            None
        }
    }
}
