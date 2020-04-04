//

use super::GenericEndpoint;
use libc;
use socket_base::Protocol;
use socket_listener::SocketListener;
use std::mem::{self, MaybeUninit};
use stream_socket::StreamSocket;

pub struct GenericStream {
    family: i32,
    protocol: i32,
}

impl GenericStream {
    pub fn new(family: i32, protocol: i32) -> Self {
        GenericStream {
            family: family,
            protocol: protocol,
        }
    }
}

impl Protocol for GenericStream {
    type Endpoint = GenericEndpoint<Self>;

    type Socket = StreamSocket<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_STREAM as _
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    fn uninit(&self) -> MaybeUninit<Self::Endpoint> {
        let data: Box<[u8]> = Box::new([0; mem::size_of::<libc::sockaddr_storage>()]);
        MaybeUninit::new(GenericEndpoint::new(data))
    }
}

pub type GenericStreamEndpoint = GenericEndpoint<GenericStream>;

pub type GenericStreamSocket = StreamSocket<GenericStream>;

pub type GenericStreamListener = SocketListener<GenericStream>;
