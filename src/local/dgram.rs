use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket, DgramSocketBuilder};
use crate::error::OsError;
use crate::local::{LocalEndpoint, LocalProtocol};
use crate::primitive::Socket;
use crate::sockaddr::AddressFamily;
use crate::socket_base::{EndpointRef, Protocol, SocketType};

/// The datagram-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalDgram;

impl LocalDgram {
    #[cfg(unix)]
    pub fn new_pair(ctx: &IoContext) -> Result<(DgramSocket<Self>, DgramSocket<Self>), OsError> {
        let (s1, s2) = Socket::pair(Self)?;
        let s1 = DgramSocket::new_impl(ctx.clone(), s1, Self);
        let s2 = DgramSocket::new_impl(ctx.clone(), s2, Self);
        Ok((s1, s2))
    }
}

impl Protocol for LocalDgram {
    type Endpoint = LocalEndpoint<Self>;
    type Type = LocalProtocol;

    fn new(_: &EndpointRef<Self::Endpoint>, _: Self::Type) -> Self {
        Self
    }

    fn family_type(self) -> AddressFamily {
        AddressFamily::AF_LOCAL
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_DGRAM
    }

    fn protocol_type(self) -> Self::Type {
        LocalProtocol
    }
}

impl DgramSocket<LocalDgram> {
    pub fn new(ctx: &IoContext) -> DgramSocketBuilder<LocalDgram> {
        DgramSocketBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

impl DgramSocketBuilder<LocalDgram> {
    pub fn unbound(self) -> Result<DgramSocket<LocalDgram>, OsError> {
        self.unbound_impl(LocalDgram)
    }
}

/// The datagram-oriented UNIX domain endpoint type.
pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

/// The datagram-oriented UNIX domain socket type.
pub type LocalDgramSocket = DgramSocket<LocalDgram>;

/// The datagram-oriented UNIX domain async socket type.
pub type AsyncLocalDgramSocket = AsyncDgramSocket<LocalDgram>;

#[test]
fn test_local_path() {
    use std::path::Path;

    let path = Path::new("/tmp/sock");
    let ep = LocalDgramEndpoint::new(path).unwrap();
    assert_eq!(&ep.as_local_addr(), path);
}

#[test]
fn test_local_abstract() {
    use std::ffi::OsStr;

    let name = OsStr::new("test");
    let ep = LocalDgramEndpoint::new(name).unwrap();
    assert_eq!(&ep.as_local_addr(), name);
}
