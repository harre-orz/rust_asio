use crate::sockaddr::{SockAddr, SockAddrWithLen, SockLen};
use std::mem::MaybeUninit;
use std::{ptr, slice};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{MAX_CONNECTIONS, Shutdown};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{MAX_CONNECTIONS, Shutdown};

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
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self.sa_ref).cast(), self.sa_len as usize) }
    }

    /// Returns owned `Endpoint`.
    pub fn clone(&self) -> E {
        unsafe {
            E::from_sockaddr(SockAddrWithLen::new_unchecked(
                self.sa_ref.clone(),
                self.sa_len,
            ))
        }
    }

    /// Returns `SockAddr` type.
    pub const fn sockaddr_ref(&self) -> &'a E::SockAddr {
        self.sa_ref
    }

    /// Returns size of `SockAddr`.
    pub const fn sockaddr_len(&self) -> SockLen {
        self.sa_len
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

/// An abstract iteration of the source or destination points.
pub trait Endpoints<'a, P>
where
    P: Protocol + 'a,
{
    type Iter: Iterator<Item = EndpointRef<'a, P::Endpoint>>;

    fn endpoints(&'a self) -> Self::Iter;
}

/// An iteration type of `*Endpoint`.
pub struct EndpointIter<'a, P: Protocol>(Option<&'a P::Endpoint>);

impl<'a, P> EndpointIter<'a, P>
where
    P: Protocol,
{
    pub fn new(ep: &'a P::Endpoint) -> Self {
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

/// An abstract socket protocol type.
pub trait Protocol: Copy {
    type Type: Copy;
    type Endpoint: Endpoint;

    fn from_endpoint(ep: &EndpointRef<Self::Endpoint>, protocol: Self::Type) -> Self;
    fn family_type(self) -> i32;
    fn socket_type(self) -> i32;
    fn protocol_type(self) -> i32;
}

/// An abstract socket option data type.
pub trait SockOpt: Sized {
    const KEY: (i32, i32);
}

/// An abstract set-able socket option data type.
pub trait SetSockOpt: SockOpt {
    fn len(&self) -> usize {
        size_of::<Self>()
    }
}

/// An abstract get-able socket option data type.
pub trait GetSockOpt: SockOpt {
    fn init(opt: MaybeUninit<Self>, _len: usize) -> Self {
        unsafe { opt.assume_init() }
    }
}

/// Socket option to allow the socket to be bound to an address that is already in use.
pub struct ReuseAddr(i32);

impl ReuseAddr {
    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(if on { 1 } else { 0 })
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl SetSockOpt for ReuseAddr {}

impl GetSockOpt for ReuseAddr {}

#[cfg(doc)]
pub use crate::socket::Socket;
