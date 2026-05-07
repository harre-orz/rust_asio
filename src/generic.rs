use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket, DgramSocketBuilder};
use crate::seqpacket_socket::{AsyncSeqPacketSocket, SeqPacketSocket, SeqPacketSocketBuilder};
use crate::sockaddr::{AddressFamily, SockAddr, SockAddrStorage, SockAddrWithLen, SockLen};
use crate::socket::{Socket, SocketType};
use crate::socket_base::{Endpoint, EndpointIter, Endpoints, Protocol};
use crate::socket_listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::stream_socket::{AsyncStreamSocket, StreamSocket, StreamSocketBuilder};
use std::fmt;
use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub struct GenericEndpoint<P> {
    ss: SockAddrStorage,
    #[cfg(not(target_os = "macos"))]
    ss_len: SockLen,
    _marker: PhantomData<P>,
}

impl<P> GenericEndpoint<P> {
    pub fn new(family_type: AddressFamily, bytes: &[u8]) -> Option<Self> {
        SockAddrStorage::new(family_type, bytes).map(|ss| {
            let (ss, ss_len) = ss.unwrap();
            Self {
                ss: ss,
                #[cfg(not(target_os = "macos"))]
                ss_len: ss_len,
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

impl<P> fmt::Debug for GenericEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GenericEndpoint {{ {:?} }}", self.as_bytes())
    }
}

impl<P> Endpoint for GenericEndpoint<P> {
    type SockAddr = SockAddrStorage;

    fn sockaddr_ref(&self) -> &Self::SockAddr {
        &self.ss
    }

    fn sockaddr_len(&self) -> SockLen {
        self.len() as SockLen
    }

    unsafe fn from_sockaddr(sa_with_len: SockAddrWithLen<Self::SockAddr>) -> Self {
        let (ss, ss_len) = sa_with_len.unwrap();
        Self {
            ss: ss,
            #[cfg(not(target_os = "macos"))]
            ss_len: ss_len,
            _marker: PhantomData,
        }
    }
}

impl<'a, P> Endpoints<'a, P> for GenericEndpoint<P>
where
    P: Protocol<Endpoint = Self> + 'a,
{
    type Iter = EndpointIter<'a, P>;

    fn endpoints(&'a self) -> Self::Iter {
        EndpointIter::new(self)
    }
}

#[derive(Copy, Clone)]
pub struct GenericDgram<T>(AddressFamily, T);

impl<T> GenericDgram<T> {
    pub const fn new(family: AddressFamily, protocol: T) -> Self {
        Self(family, protocol)
    }
}

impl<T> Protocol for GenericDgram<T>
where
    T: Copy + Into<i32>,
{
    type Type = T;
    type Endpoint = GenericEndpoint<Self>;

    fn new(address_family: AddressFamily, protocol: Self::Type) -> Self {
        Self(address_family, protocol)
    }

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_DGRAM
    }

    fn protocol_type(self) -> T {
        self.1
    }
}

impl<T> GenericEndpoint<GenericDgram<T>>
where
    T: Copy + Into<i32>,
{
    pub fn protocol(&self, pro: <GenericDgram<T> as Protocol>::Type) -> GenericDgram<T> {
        GenericDgram(self.ss.address_family(), pro)
    }
}

impl<T> DgramSocket<GenericDgram<T>>
where
    T: Copy + Into<i32>,
{
    pub fn new(
        ctx: &IoContext,
        pro: <GenericDgram<T> as Protocol>::Type,
    ) -> DgramSocketBuilder<GenericDgram<T>> {
        DgramSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

pub type GenericDgramEndpoint<T> = GenericEndpoint<GenericDgram<T>>;
pub type GenericDgramSocket<T> = DgramSocket<GenericDgram<T>>;
pub type AsyncGenericDgramSocket<T> = AsyncDgramSocket<GenericDgram<T>>;

#[derive(Copy, Clone)]
pub struct GenericRaw<T>(AddressFamily, T);

impl<T> GenericRaw<T> {
    pub const fn new(family: AddressFamily, protocol: T) -> Self {
        Self(family, protocol)
    }
}

impl<T> Protocol for GenericRaw<T>
where
    T: Copy + Into<i32>,
{
    type Type = T;
    type Endpoint = GenericEndpoint<Self>;

    fn new(address_family: AddressFamily, protocol: Self::Type) -> Self {
        Self(address_family, protocol)
    }

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_RAW
    }

    fn protocol_type(self) -> T {
        self.1
    }
}

impl<T> GenericEndpoint<GenericRaw<T>>
where
    T: Copy + Into<i32>,
{
    pub fn protocol(&self, pro: <GenericRaw<T> as Protocol>::Type) -> GenericRaw<T> {
        GenericRaw(self.ss.address_family(), pro)
    }
}

impl<T> DgramSocket<GenericRaw<T>>
where
    T: Copy + Into<i32>,
{
    pub fn new(
        ctx: &IoContext,
        pro: <GenericRaw<T> as Protocol>::Type,
    ) -> DgramSocketBuilder<GenericRaw<T>> {
        DgramSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

pub type GenericRawEndpoint<T> = GenericEndpoint<GenericRaw<T>>;
pub type GenericRawSocket<T> = DgramSocket<GenericRaw<T>>;
pub type AsyncGenericRawSocket<T> = AsyncDgramSocket<GenericRaw<T>>;

#[derive(Copy, Clone)]
pub struct GenericStream<T>(AddressFamily, T);

impl<T> GenericStream<T> {
    pub const fn new(family: AddressFamily, protocol: T) -> Self {
        Self(family, protocol)
    }
}

impl<T> Protocol for GenericStream<T>
where
    T: Copy + Into<i32>,
{
    type Type = T;
    type Endpoint = GenericEndpoint<Self>;

    fn new(address_family: AddressFamily, protocol: Self::Type) -> Self {
        Self(address_family, protocol)
    }

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_STREAM
    }

    fn protocol_type(self) -> T {
        self.1
    }
}

impl<T> GenericEndpoint<GenericStream<T>>
where
    T: Copy + Into<i32>,
{
    pub fn protocol(&self, pro: <GenericStream<T> as Protocol>::Type) -> GenericStream<T> {
        GenericStream(self.ss.address_family(), pro)
    }
}

impl<T> StreamSocket<GenericStream<T>>
where
    T: Copy + Into<i32>,
{
    pub fn new(
        ctx: &IoContext,
        pro: <GenericStream<T> as Protocol>::Type,
    ) -> StreamSocketBuilder<GenericStream<T>> {
        StreamSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

impl<T> ConnectedSocket for SocketListener<GenericStream<T>>
where
    T: Copy + Into<i32>,
{
    type Socket = StreamSocket<GenericStream<T>>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_impl(self.as_ctx().clone(), soc)
    }
}

impl<T> ConnectedSocket for AsyncSocketListener<GenericStream<T>>
where
    T: Copy + Into<i32>,
{
    type Socket = AsyncStreamSocket<GenericStream<T>>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_impl(self.as_ctx().clone(), soc).into()
    }
}

pub type GenericStreamEndpoint<T> = GenericEndpoint<GenericStream<T>>;
pub type GenericStreamSocket<T> = DgramSocket<GenericStream<T>>;
pub type GenericStreamListener<T> = SocketListener<GenericStream<T>>;
pub type AsyncGenericStreamSocket<T> = AsyncDgramSocket<GenericStream<T>>;
pub type AsyncGenericStreamListener<T> = SocketListener<GenericStream<T>>;

#[derive(Copy, Clone)]
pub struct GenericSeqPacket<T>(AddressFamily, T);

impl<T> GenericSeqPacket<T> {
    pub const fn new(family: AddressFamily, protocol: T) -> Self {
        Self(family, protocol)
    }
}

impl<T> Protocol for GenericSeqPacket<T>
where
    T: Copy + Into<i32>,
{
    type Type = T;
    type Endpoint = GenericEndpoint<Self>;

    fn new(address_family: AddressFamily, protocol: Self::Type) -> Self {
        Self(address_family, protocol)
    }

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_SEQPACKET
    }

    fn protocol_type(self) -> T {
        self.1
    }
}

impl<T> GenericEndpoint<GenericSeqPacket<T>>
where
    T: Copy + Into<i32>,
{
    pub fn protocol(&self, pro: <GenericSeqPacket<T> as Protocol>::Type) -> GenericSeqPacket<T> {
        GenericSeqPacket(self.ss.address_family(), pro)
    }
}

impl<T> SeqPacketSocket<GenericSeqPacket<T>>
where
    T: Copy + Into<i32>,
{
    pub fn new(
        ctx: &IoContext,
        pro: <GenericSeqPacket<T> as Protocol>::Type,
    ) -> SeqPacketSocketBuilder<GenericSeqPacket<T>> {
        SeqPacketSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

impl<T> ConnectedSocket for SocketListener<GenericSeqPacket<T>>
where
    T: Copy + Into<i32>,
{
    type Socket = SeqPacketSocket<GenericSeqPacket<T>>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        Self::Socket::new_impl(self.as_ctx().clone(), soc)
    }
}

impl<T> ConnectedSocket for AsyncSocketListener<GenericSeqPacket<T>>
where
    T: Copy + Into<i32>,
{
    type Socket = AsyncSeqPacketSocket<GenericSeqPacket<T>>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_impl(self.as_ctx().clone(), soc).into()
    }
}

pub type GenericSeqPacketEndpoint<T> = GenericEndpoint<GenericSeqPacket<T>>;
pub type GenericSeqPacketSocket<T> = DgramSocket<GenericSeqPacket<T>>;
pub type GenericSeqPacketListener<T> = SocketListener<GenericSeqPacket<T>>;
pub type AsyncGenericSeqPacketSocket<T> = AsyncDgramSocket<GenericSeqPacket<T>>;
pub type AsyncGenericSeqPacketListener<T> = SocketListener<GenericSeqPacket<T>>;
