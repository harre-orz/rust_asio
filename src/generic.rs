use crate::ffi::sockaddr::SockAddrStorage;
use crate::ffi::socket::Socket;
use crate::seqpacket_socket::{AsyncSeqPacketSocket, SeqPacketSocket};
use crate::socket_base::{AddressFamily, Endpoint, IntoProtocolType, Protocol, SocketType};
use crate::socket_listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::stream_socket::{AsyncStreamSocket, StreamSocket};
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

pub struct GenericStream<P>(AddressFamily, P);

impl<P> Clone for GenericStream<P>
where
    P: IntoProtocolType,
{
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<P> Copy for GenericStream<P> where P: IntoProtocolType {}

impl<P> GenericStream<P>
where
    P: IntoProtocolType,
{
    pub const fn new(family: AddressFamily, protocol: P) -> Self {
        Self(family, protocol)
    }
}

impl<P> Protocol for GenericStream<P>
where
    P: IntoProtocolType,
{
    type Type = P;
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::STREAM
    }

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}

impl<P> ConnectedSocket for SocketListener<GenericStream<P>>
where
    P: IntoProtocolType,
{
    type Socket = StreamSocket<GenericStream<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl<P> ConnectedSocket for AsyncSocketListener<GenericStream<P>>
where
    P: IntoProtocolType,
{
    type Socket = AsyncStreamSocket<GenericStream<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

pub struct GenericDgram<P>(AddressFamily, P);

impl<P: IntoProtocolType> Clone for GenericDgram<P> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<P: IntoProtocolType> Copy for GenericDgram<P> {}

impl<P: IntoProtocolType> GenericDgram<P> {
    pub const fn new(family: AddressFamily, protocol: P) -> Self {
        Self(family, protocol)
    }
}

impl<P> Protocol for GenericDgram<P>
where
    P: IntoProtocolType,
{
    type Type = P;
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::DGRAM
    }

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}

pub struct GenericSeqPacket<P>(AddressFamily, P);

impl<P: IntoProtocolType> Clone for GenericSeqPacket<P> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<P: IntoProtocolType> Copy for GenericSeqPacket<P> {}

impl<P: IntoProtocolType> GenericSeqPacket<P> {
    pub const fn new(family: AddressFamily, protocol: P) -> Self {
        Self(family, protocol)
    }
}

impl<P> Protocol for GenericSeqPacket<P>
where
    P: IntoProtocolType,
{
    type Type = P;
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::RAW
    }

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}

impl<P> ConnectedSocket for SeqPacketSocket<GenericSeqPacket<P>>
where
    P: IntoProtocolType,
{
    type Socket = SeqPacketSocket<GenericSeqPacket<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl<P> ConnectedSocket for AsyncSocketListener<GenericSeqPacket<P>>
where
    P: IntoProtocolType,
{
    type Socket = AsyncSeqPacketSocket<GenericSeqPacket<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

pub struct GenericRaw<P>(AddressFamily, P);

impl<P: IntoProtocolType> Clone for GenericRaw<P> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<P: IntoProtocolType> Copy for GenericRaw<P> {}

impl<P: IntoProtocolType> GenericRaw<P> {
    pub const fn new(family: AddressFamily, protocol: P) -> Self {
        Self(family, protocol)
    }
}

impl<P> Protocol for GenericRaw<P>
where
    P: IntoProtocolType,
{
    type Type = P;
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::RAW
    }

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}
