use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket, DgramSocketBuilder};
use crate::error::OsError;
use crate::seqpacket_socket::{AsyncSeqPacketSocket, SeqPacketSocket, SeqPacketSocketBuilder};
use crate::sockaddr::{AddressFamily, SockAddrUnix, SockAddrWithLen, SockLen};
use crate::socket::{Socket, SocketType};
use crate::socket_base::{Endpoint, EndpointIter, EndpointRef, Endpoints, Protocol};
use crate::socket_listener::{
    AsyncSocketListener, ConnectedSocket, SocketListener, SocketListenerBuilder,
};
use crate::stream_socket::{AsyncStreamSocket, StreamSocket, StreamSocketBuilder};
use std::ffi::OsStr;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::{fmt, slice};

#[derive(Eq, PartialEq, Debug)]
pub enum LocalAddrRef<'a> {
    Path(&'a Path),
    Abstract(&'a OsStr),
}

impl<'a> PartialEq<Path> for LocalAddrRef<'a> {
    fn eq(&self, other: &Path) -> bool {
        match self {
            &Self::Path(path) => path == other,
            _ => false,
        }
    }
}

impl<'a> PartialEq<OsStr> for LocalAddrRef<'a> {
    fn eq(&self, other: &OsStr) -> bool {
        match self {
            &Self::Abstract(name) => name == other,
            _ => false,
        }
    }
}

pub trait AsLocalAddr<'a> {
    fn as_local_addr(&self) -> LocalAddrRef<'a>;
}

impl<'a> AsLocalAddr<'a> for LocalAddrRef<'a> {
    fn as_local_addr(&self) -> Self {
        match self {
            LocalAddrRef::Path(path) => LocalAddrRef::Path(path),
            LocalAddrRef::Abstract(name) => LocalAddrRef::Abstract(name),
        }
    }
}

impl<'a> AsLocalAddr<'a> for &'a LocalAddrRef<'a> {
    fn as_local_addr(&self) -> LocalAddrRef<'a> {
        match self {
            LocalAddrRef::Path(path) => LocalAddrRef::Path(path),
            LocalAddrRef::Abstract(name) => LocalAddrRef::Abstract(name),
        }
    }
}

impl<'a> AsLocalAddr<'a> for &'a Path {
    fn as_local_addr(&self) -> LocalAddrRef<'a> {
        LocalAddrRef::Path(self)
    }
}

impl<'a> AsLocalAddr<'a> for &'a PathBuf {
    fn as_local_addr(&self) -> LocalAddrRef<'a> {
        LocalAddrRef::Path(self)
    }
}

impl<'a> AsLocalAddr<'a> for &'a OsStr {
    fn as_local_addr(&self) -> LocalAddrRef<'a> {
        LocalAddrRef::Abstract(self)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[non_exhaustive]
pub struct LocalProtocol;

#[derive(Copy, Clone)]
pub struct LocalEndpoint<P> {
    sun: SockAddrUnix,
    #[cfg(not(target_os = "macos"))]
    sun_len: SockLen,
    _marker: PhantomData<P>,
}

impl<P> LocalEndpoint<P> {
    pub fn new<'a, T>(local_addr: T) -> Result<Self, OsError>
    where
        T: AsLocalAddr<'a>,
    {
        let sun = match local_addr.as_local_addr() {
            LocalAddrRef::Path(path) => {
                SockAddrUnix::new(path.as_os_str().as_encoded_bytes(), false)
            }
            LocalAddrRef::Abstract(name) => SockAddrUnix::new(name.as_encoded_bytes(), true),
        };
        let (sun, sun_len) = sun?.unwrap();
        Ok(Self {
            sun: sun,
            #[cfg(not(target_os = "macos"))]
            sun_len: sun_len,
            _marker: PhantomData,
        })
    }

    #[cfg(not(target_os = "macos"))]
    pub const fn len(&self) -> SockLen {
        self.sun_len
    }
    #[cfg(target_os = "macos")]
    pub const fn len(&self) -> SockLen {
        self.sun.len() as SockLen
    }

    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { self.sun.as_bytes_unchecked(self.len()) }
    }

    pub fn as_local_addr(&self) -> LocalAddrRef<'_> {
        let bytes = self.as_bytes();
        if bytes[2] != 0 {
            let bytes = &bytes[2..];
            unsafe {
                let bytes = slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len());
                LocalAddrRef::Path(Path::new(OsStr::from_encoded_bytes_unchecked(bytes)))
            }
        } else {
            let bytes = &bytes[3..];
            unsafe {
                let bytes = slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len());
                LocalAddrRef::Abstract(OsStr::from_encoded_bytes_unchecked(bytes))
            }
        }
    }
}

impl<P> fmt::Debug for LocalEndpoint<P>
where
    P: Protocol,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_local_addr() {
            LocalAddrRef::Path(path) => write!(f, "LocalEndpoint {{ Path({:?}) }}", path),
            LocalAddrRef::Abstract(name) => write!(f, "LocalEndpoint {{ Abstract({:?}) }}", name),
        }
    }
}

impl<P> Endpoint for LocalEndpoint<P>
where
    P: Protocol,
{
    type SockAddr = SockAddrUnix;

    fn sockaddr_ref(&self) -> &Self::SockAddr {
        &self.sun
    }

    fn sockaddr_len(&self) -> SockLen {
        self.len()
    }

    unsafe fn from_sockaddr(sa_with_len: SockAddrWithLen<Self::SockAddr>) -> Self {
        let (sun, sun_len) = sa_with_len.unwrap();
        Self {
            sun: sun,
            #[cfg(not(target_os = "macos"))]
            sun_len: sun_len,
            _marker: PhantomData,
        }
    }
}

impl<'a, P> Endpoints<'a, P> for LocalEndpoint<P>
where
    P: Protocol<Endpoint = Self> + 'a,
{
    type Iter = EndpointIter<'a, P>;

    fn endpoints(&'a self) -> Self::Iter {
        EndpointIter::new(self)
    }
}

/// The datagram-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalDgram;

impl LocalDgram {
    #[cfg(unix)]
    pub fn new_pair(ctx: &IoContext) -> Result<(DgramSocket<Self>, DgramSocket<Self>), OsError> {
        let (s1, s2) = Socket::socketpair(Self)?;
        let s1 = DgramSocket::new_impl(ctx.clone(), s1, Self);
        let s2 = DgramSocket::new_impl(ctx.clone(), s2, Self);
        Ok((s1, s2))
    }
}

impl Protocol for LocalDgram {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn from_endpoint(_: &EndpointRef<Self::Endpoint>, _: Self::Type) -> Self {
        Self
    }

    fn family_type(self) -> i32 {
        AddressFamily::AF_LOCAL.into()
    }

    fn socket_type(self) -> i32 {
        SocketType::SOCK_DGRAM
    }

    fn protocol_type(self) -> i32 {
        0
    }
}

impl DgramSocket<LocalDgram> {
    pub fn new(ctx: &IoContext) -> DgramSocketBuilder<LocalDgram> {
        DgramSocketBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

/// The datagram-oriented UNIX domain endpoint type.
pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

/// The datagram-oriented UNIX domain socket type.
pub type LocalDgramSocket = DgramSocket<LocalDgram>;

/// The datagram-oriented UNIX domain async socket type.
pub type AsyncLocalDgramSocket = AsyncDgramSocket<LocalDgram>;

/// The stream-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalStream;

impl LocalStream {
    #[cfg(unix)]
    pub fn new_pair(ctx: &IoContext) -> Result<(StreamSocket<Self>, StreamSocket<Self>), OsError> {
        let (s1, s2) = Socket::socketpair(Self)?;
        Ok((
            StreamSocket::new_impl(ctx.clone(), s1, Self),
            StreamSocket::new_impl(ctx.clone(), s2, Self),
        ))
    }
}

impl Protocol for LocalStream {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn from_endpoint(_: &EndpointRef<Self::Endpoint>, _: Self::Type) -> Self {
        LocalStream
    }

    fn family_type(self) -> i32 {
        AddressFamily::AF_LOCAL.into()
    }

    fn socket_type(self) -> i32 {
        SocketType::SOCK_STREAM
    }

    fn protocol_type(self) -> i32 {
        0
    }
}

impl StreamSocket<LocalStream> {
    pub fn new(ctx: &IoContext) -> StreamSocketBuilder<LocalStream> {
        StreamSocketBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

impl SocketListener<LocalStream> {
    pub fn new(ctx: &IoContext) -> SocketListenerBuilder<LocalStream> {
        SocketListenerBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

impl ConnectedSocket for AsyncSocketListener<LocalStream> {
    type Socket = AsyncStreamSocket<LocalStream>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_impl(self.as_ctx().clone(), soc, self.protocol()).into()
    }
}

impl ConnectedSocket for SocketListener<LocalStream> {
    type Socket = StreamSocket<LocalStream>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        Self::Socket::new_impl(self.as_ctx().clone(), soc, self.protocol())
    }
}

/// The stream-oriented UNIX domain endpoint type
pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

/// The stream-oriented UNIX domain socket type
pub type LocalStreamSocket = StreamSocket<LocalStream>;

/// The stream-oriented UNIX domain listener type
pub type LocalStreamListener = SocketListener<LocalStream>;

/// The stream-oriented UNIX domain async socket type
pub type AsyncLocalStreamSocket = AsyncStreamSocket<LocalStream>;

/// The stream-oriented UNIX domain async listener type
pub type AsyncLocalStreamListener = AsyncSocketListener<LocalStream>;

/// The seq-packet protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalSeqPacket;

impl LocalSeqPacket {
    #[cfg(unix)]
    pub fn new_pair(
        ctx: &IoContext,
    ) -> Result<(SeqPacketSocket<Self>, SeqPacketSocket<Self>), OsError> {
        let (s1, s2) = Socket::socketpair(Self)?;
        Ok((
            SeqPacketSocket::new_impl(ctx.clone(), s1, Self),
            SeqPacketSocket::new_impl(ctx.clone(), s2, Self),
        ))
    }
}

impl Protocol for LocalSeqPacket {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn from_endpoint(_: &EndpointRef<Self::Endpoint>, _: Self::Type) -> Self {
        Self
    }

    fn family_type(self) -> i32 {
        AddressFamily::AF_LOCAL.into()
    }

    fn socket_type(self) -> i32 {
        SocketType::SOCK_SEQPACKET
    }

    fn protocol_type(self) -> i32 {
        0
    }
}

impl SeqPacketSocket<LocalSeqPacket> {
    pub fn new(ctx: &IoContext) -> SeqPacketSocketBuilder<LocalSeqPacket> {
        SeqPacketSocketBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

impl SocketListener<LocalSeqPacket> {
    pub fn new(ctx: &IoContext) -> SocketListenerBuilder<LocalSeqPacket> {
        SocketListenerBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

impl ConnectedSocket for SocketListener<LocalSeqPacket> {
    type Socket = SeqPacketSocket<LocalSeqPacket>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        Self::Socket::new_impl(self.as_ctx().clone(), soc, self.protocol())
    }
}

impl ConnectedSocket for AsyncSocketListener<LocalSeqPacket> {
    type Socket = AsyncSeqPacketSocket<LocalSeqPacket>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_impl(self.as_ctx().clone(), soc, self.protocol()).into()
    }
}

/// The seq-packet-oriented UNIX domain endpoint type.
pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain socket type.
pub type LocalSeqPacketSocket = SeqPacketSocket<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain listener type.
pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain async socket type.
pub type AsyncLocalSeqPacketSocket = AsyncSeqPacketSocket<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain async listener type.
pub type AsyncLocalSeqPacketListener = AsyncSocketListener<LocalSeqPacket>;

#[test]
fn test_local_path() {
    let path = Path::new("/tmp/sock");
    let ep = LocalDgramEndpoint::new(path).unwrap();
    assert_eq!(&ep.as_local_addr(), path);
}

#[test]
fn test_local_abstract() {
    let name = OsStr::new("test");
    let ep = LocalDgramEndpoint::new(name).unwrap();
    assert_eq!(&ep.as_local_addr(), name);
}
