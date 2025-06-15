use crate::IoContext;
use crate::dgram_socket::DgramSocket;
use crate::error::OsError;
use crate::ffi::sockaddr::SockAddrUnix;
use crate::ffi::socket::Socket;
use crate::seqpacket_socket::{AsyncSeqPacketSocket, SeqPacketSocket};
use crate::socket_base::{AddressFamily, Endpoint, IntoProtocolType, Protocol, SocketType};
use crate::socket_listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::stream_socket::{AsyncStreamSocket, StreamSocket};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::str;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalProtocol;

impl Into<i32> for LocalProtocol {
    fn into(self) -> i32 {
        0
    }
}

impl IntoProtocolType for LocalProtocol {}

#[derive(Clone, Debug)]
pub enum LocalAddr {
    Path(PathBuf),
    Abstract(String),
    Unnamed,
}

#[derive(Copy, Clone)]
pub struct LocalEndpoint<P> {
    inner: SockAddrUnix,
    _marker: PhantomData<P>,
}

impl<P> LocalEndpoint<P> {
    pub fn new<T>(addr: &LocalAddr) -> Result<Self, OsError> {
        match addr {
            LocalAddr::Path(path) => Self::new_path(path),
            LocalAddr::Abstract(name) => Self::new_abstract(name),
            LocalAddr::Unnamed => Ok(Self::new_unnamed()),
        }
    }

    pub fn new_path<T>(path: T) -> Result<Self, OsError>
    where
        T: AsRef<Path>,
    {
        Ok(Self {
            inner: SockAddrUnix::new_path(path.as_ref())?,
            _marker: PhantomData,
        })
    }

    pub fn new_abstract<T>(name: T) -> Result<Self, OsError>
    where
        T: AsRef<str>,
    {
        Ok(Self {
            inner: SockAddrUnix::new_abstract(name.as_ref())?,
            _marker: PhantomData,
        })
    }

    pub const fn new_unnamed() -> Self {
        Self {
            inner: SockAddrUnix::new_unnamed(),
            _marker: PhantomData,
        }
    }

    fn as_path(&self) -> Option<&Path> {
        self.inner.as_path()
    }

    fn as_abstract(&self) -> Option<&str> {
        self.inner.as_abstract()
    }

    pub const fn is_unnamed(&self) -> bool {
        self.inner.is_unnamed()
    }

    pub const fn family_type(&self) -> AddressFamily {
        self.inner.family_type()
    }

    pub fn addr(&self) -> LocalAddr {
        if let Some(path) = self.as_path() {
            LocalAddr::Path(path.into())
        } else if let Some(name) = self.as_abstract() {
            LocalAddr::Abstract(name.into())
        } else {
            LocalAddr::Unnamed
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

impl<P> Endpoint for LocalEndpoint<P>
where
    P: Protocol,
{
    type SockAddr = SockAddrUnix;

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

// impl<P> Endpoint for LocalEndpoint<P>
// where
//     P: Protocol,
// {
//     const MAX_SIZE: SocklenType = SIZE_OF_SOCKADDR_UN;
//
//     fn as_ptr(&self) -> SockaddrType {
//         self.inner.as_ptr()
//     }
//
//     fn len(&self) -> SocklenType {
//         self.inner.len()
//     }
//
//     unsafe fn init(ep: MaybeUninit<Self>, len: SocklenType) -> Self {
//         panic!()
//         // if len >= Self::SIZE {
//         //     panic!()
//         // }
//         //
//         // let mut ep = ep.assume_init();
//         // ep.len = len;
//         // if ep.is_unnamed() {
//         //     ep
//         // } else if let Some(_) = ep.as_path() {
//         //     ep
//         // } else if let Some(_) = ep.as_abstract() {
//         //     ep
//         // } else {
//         //     panic!()
//         // }
//     }
// }

/// The stream-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalStream;

impl LocalStream {
    pub fn connect(
        ctx: &IoContext,
        ep: &LocalEndpoint<Self>,
    ) -> Result<StreamSocket<Self>, OsError> {
        let soc = StreamSocket::new(ctx, Self)?;
        soc.connect(ep)
    }

    #[cfg(unix)]
    pub fn socketpair(
        ctx: &IoContext,
    ) -> Result<(StreamSocket<Self>, StreamSocket<Self>), OsError> {
        let (s1, s2) = Socket::socketpair(Self)?;
        Ok((
            StreamSocket::new_priv(ctx, s1, Self),
            StreamSocket::new_priv(ctx, s2, Self),
        ))
    }
}

impl Protocol for LocalStream {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        AddressFamily::UNIX
    }

    fn socket_type(self) -> SocketType {
        SocketType::STREAM
    }

    fn protocol_type(self) -> Self::Type {
        LocalProtocol
    }
}

impl ConnectedSocket for SocketListener<LocalStream> {
    type Socket = StreamSocket<LocalStream>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        Self::Socket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl ConnectedSocket for AsyncSocketListener<LocalStream> {
    type Socket = AsyncStreamSocket<LocalStream>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        StreamSocket::<LocalStream>::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

/// The stream-oriented UNIX domain endpoint type
pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

/// The stream-oriented UNIX domain socket type
pub type LocalStreamSocket = StreamSocket<LocalStream>;

/// The stream-oriented UNIX domain listener type
pub type LocalStreamListener = SocketListener<LocalStream>;

/// The datagram-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalDgram;

impl LocalDgram {
    pub fn connect(
        ctx: &IoContext,
        ep: &LocalEndpoint<Self>,
    ) -> Result<DgramSocket<Self>, OsError> {
        let soc = DgramSocket::new(ctx, Self)?;
        soc.connect(ep)?;
        Ok(soc)
    }

    #[cfg(unix)]
    pub fn socketpair(ctx: &IoContext) -> Result<(DgramSocket<Self>, DgramSocket<Self>), OsError> {
        let (s1, s2) = Socket::socketpair(Self)?;
        Ok((
            DgramSocket::new_priv(ctx, s1, Self),
            DgramSocket::new_priv(ctx, s2, Self),
        ))
    }
}

impl Protocol for LocalDgram {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        AddressFamily::UNIX
    }

    fn socket_type(self) -> SocketType {
        SocketType::DGRAM
    }

    fn protocol_type(self) -> Self::Type {
        LocalProtocol
    }
}

/// The datagram-oriented UNIX domain endpoint type.
pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

/// The datagram-oriented UNIX domain socket type.
pub type LocalDgramSocket = DgramSocket<LocalDgram>;

/// The seq-packet protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalSeqPacket;

impl LocalSeqPacket {
    pub fn connect(
        ctx: &IoContext,
        ep: &LocalEndpoint<Self>,
    ) -> Result<SeqPacketSocket<Self>, OsError> {
        let soc = SeqPacketSocket::new(ctx, Self)?;
        soc.connect(ep)
    }

    #[cfg(unix)]
    pub fn socketpair(
        ctx: &IoContext,
    ) -> Result<(SeqPacketSocket<Self>, SeqPacketSocket<Self>), OsError> {
        let (s1, s2) = Socket::socketpair(Self)?;
        Ok((
            SeqPacketSocket::new_priv(ctx, s1, Self),
            SeqPacketSocket::new_priv(ctx, s2, Self),
        ))
    }
}

impl Protocol for LocalSeqPacket {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        AddressFamily::UNIX
    }

    fn socket_type(self) -> SocketType {
        SocketType::SEQPACKET
    }

    fn protocol_type(self) -> Self::Type {
        LocalProtocol
    }
}

impl ConnectedSocket for SocketListener<LocalSeqPacket> {
    type Socket = SeqPacketSocket<LocalSeqPacket>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        Self::Socket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl ConnectedSocket for AsyncSocketListener<LocalSeqPacket> {
    type Socket = AsyncSeqPacketSocket<LocalSeqPacket>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

/// The seq-packet-oriented UNIX domain endpoint type.
pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain socket type.
pub type LocalSeqPacketSocket = SeqPacketSocket<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain listener type.
pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket>;
