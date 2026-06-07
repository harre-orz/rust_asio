use crate::sockaddr::{AddressFamily, SockAddr, SockAddrWithLen, SockLen};
use std::marker::PhantomData;
use std::{mem, ptr, slice};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock;

pub const MAX_CONNECTIONS: i32 = {
    #[cfg(unix)]
    let max_conn = libc::SOMAXCONN;
    #[cfg(windows)]
    let max_conn = WinSock::SOMAXCONN as i32;
    max_conn
};

/// Possible values which can be passed to the shutdown method.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(i32)]
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = {
        #[cfg(unix)]
        let read = libc::SHUT_RD;
        #[cfg(windows)]
        let read = WinSock::SD_RECEIVE;
        read
    },

    /// Indicates that the writing portion of this socket should be shut down.
    Write = {
        #[cfg(unix)]
        let write = libc::SHUT_WR;
        #[cfg(windows)]
        let write = WinSock::SD_SEND;
        write
    },

    /// Shut down both the reading and writing portions of this socket.
    Both = {
        #[cfg(unix)]
        let both = libc::SHUT_RDWR;
        #[cfg(windows)]
        let both = WinSock::SD_BOTH;
        both
    },
}

pub struct SocketType(i32);

impl SocketType {
    pub const SOCK_STREAM: Self = Self({
        #[cfg(unix)]
        let stream = libc::SOCK_STREAM;
        #[cfg(windows)]
        let stream = WinSock::SOCK_STREAM;
        stream
    });

    pub const SOCK_DGRAM: Self = Self({
        #[cfg(unix)]
        let dgram = libc::SOCK_DGRAM;
        #[cfg(windows)]
        let dgram = WinSock::SOCK_DGRAM;
        dgram
    });

    pub const SOCK_RAW: Self = Self({
        #[cfg(unix)]
        let raw = libc::SOCK_RAW;
        #[cfg(windows)]
        let raw = WinSock::SOCK_RAW;
        raw
    });

    pub const SOCK_SEQPACKET: Self = Self({
        #[cfg(unix)]
        let seqpacket = libc::SOCK_SEQPACKET;
        #[cfg(windows)]
        let seqpacket = WinSock::SOCK_SEQPACKET;
        seqpacket
    });
}

impl Into<i32> for SocketType {
    fn into(self) -> i32 {
        self.0
    }
}

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

    #[cfg(windows)]
    pub(crate) fn unspecified(&self) -> E {
        unsafe {
            let mut sa = mem::zeroed::<<E as Endpoint>::SockAddr>();
            (*(ptr::from_mut(&mut sa) as *mut WinSock::SOCKADDR)).sa_family = (*(ptr::from_ref(self.sa_ref) as *const WinSock::SOCKADDR)).sa_family;
            E::from_sockaddr(SockAddrWithLen::new_unchecked(sa, self.sa_len))
        }
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
pub trait Protocol: Copy + 'static {
    type Endpoint: Endpoint;
    type Type: Copy + Into<i32> + 'static;

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
