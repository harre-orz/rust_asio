//

use super::LocalEndpoint;
use libc;
use socket_base::Protocol;
use socket_listener::SocketListener;
use std::mem::MaybeUninit;
use stream_socket::StreamSocket;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalStream;

impl LocalEndpoint<LocalStream> {
    pub fn protocol(&self) -> LocalStream {
        LocalStream
    }
}

impl Protocol for LocalStream {
    type Endpoint = LocalEndpoint<Self>;
    type Socket = StreamSocket<Self>;

    fn family_type(&self) -> i32 {
        libc::AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_STREAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    fn uninit(&self) -> MaybeUninit<Self::Endpoint> {
        MaybeUninit::uninit()
    }
}

pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

pub type LocalStreamSocket = StreamSocket<LocalStream>;

pub type LocalStreamListener = SocketListener<LocalStream>;
