use ffi::{SOCK_DGRAM, socklen_t};
use prelude::{Endpoint, Protocol};
use generic::{GenericEndpoint};
use dgram_socket::DgramSocket;
use socket_builder::SocketBuilder;
use socket_base::{Tx, Rx};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GenericDgram {
    family: i32,
    protocol: i32,
    capacity: socklen_t,
}

impl GenericEndpoint<GenericDgram> {
    pub fn protocol(&self) -> GenericDgram {
        GenericDgram {
            family: unsafe { &*self.as_ptr() }.sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }
}

impl Protocol for GenericDgram {
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        GenericEndpoint::default(self.capacity, self.protocol)
    }
}

pub type GenericDgramEndpoint = GenericEndpoint<GenericDgram>;

pub type GenericDgramBuilder = SocketBuilder<GenericDgram, DgramSocket<GenericDgram, Tx>, DgramSocket<GenericDgram, Rx>>;

pub type GenericDgramRxSocket = DgramSocket<GenericDgram, Rx>;

pub type GenericDgramTxSocket = DgramSocket<GenericDgram, Tx>;
