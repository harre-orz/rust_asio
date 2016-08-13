use {Protocol, StreamSocket, SocketListener};
use backbone::{AF_LOCAL, SOCK_STREAM};
use super::LocalEndpoint;

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
}

impl LocalEndpoint<LocalStream> {
    pub fn protocol(&self) -> LocalStream {
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
