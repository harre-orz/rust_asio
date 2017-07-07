use ffi::{SOCK_STREAM, socklen_t};
use prelude::{Endpoint, Protocol};
use generic::{GenericEndpoint};
use stream_socket::StreamSocket;
use socket_builder::SocketBuilder;
use socket_listener::SocketListener;
use socket_base::{Tx, Rx};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GenericStream {
    family: i32,
    protocol: i32,
    capacity: socklen_t,
}

impl GenericEndpoint<GenericStream> {
    pub fn protocol(&self) -> GenericStream {
        GenericStream {
            family: unsafe { &*self.as_ptr() }.sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }
}

impl Protocol for GenericStream {
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        GenericEndpoint::default(self.capacity, self.protocol)
    }
}

pub type GenericStreamEndpoint = GenericEndpoint<GenericStream>;

pub type GenericStreamBuilder = SocketBuilder<GenericStream, StreamSocket<GenericStream, Tx>, StreamSocket<GenericStream, Rx>>;

pub type GenericStreamListener = SocketListener<GenericStream, StreamSocket<GenericStream, Tx>, StreamSocket<GenericStream, Rx>>;

pub type GenericStreamRxSocket = StreamSocket<GenericStream, Rx>;

pub type GenericStreamTxSocket = StreamSocket<GenericStream, Tx>;
