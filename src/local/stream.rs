use std::mem;
use {Protocol, Endpoint, StreamSocket, SocketListener};
use backbone::{AF_LOCAL, SOCK_STREAM};
use super::{LocalProtocol, LocalEndpoint};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalStream;

impl Protocol for LocalStream {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl LocalProtocol for LocalStream {
}

impl Endpoint<LocalStream> for LocalEndpoint<LocalStream> {
    fn protocol(&self) -> LocalStream {
        LocalStream
    }
}

pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

pub type LocalStreamSocket = StreamSocket<LocalStream>;

pub type LocalStreamListener = SocketListener<LocalStream>;

#[test]
fn test_stream() {
    assert!(LocalStream == LocalStream);
}

#[test]
fn test_getsockname_local() {
    use IoService;
    use super::*;
    use std::fs;

    let io = IoService::new();
    let soc = LocalStreamSocket::new(&io, LocalStream).unwrap();
    let ep = LocalStreamEndpoint::new("/tmp/asio_foo.sock").unwrap();
    let _ = fs::remove_file(ep.path());
    soc.bind(&ep).unwrap();
    assert_eq!(soc.local_endpoint().unwrap(), ep);
    let _ = fs::remove_file(ep.path());
}
