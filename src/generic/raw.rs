use ffi::{SOCK_RAW, socklen_t};
use prelude::{Endpoint, Protocol};
use generic::{GenericEndpoint};
use dgram_socket::DgramSocket;
use socket_builder::SocketBuilder;
use socket_base::{Tx, Rx};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GenericRaw {
    family: i32,
    protocol: i32,
    capacity: socklen_t,
}

impl GenericEndpoint<GenericRaw> {
    pub fn protocol(&self) -> GenericRaw {
        GenericRaw {
            family: unsafe { &*self.as_ptr() }.sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }
}

impl Protocol for GenericRaw {
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_RAW
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        GenericEndpoint::default(self.capacity, self.protocol)
    }
}

pub type GenericRawEndpoint = GenericEndpoint<GenericRaw>;

pub type GenericRawBuilder = SocketBuilder<GenericRaw, DgramSocket<GenericRaw, Tx>, DgramSocket<GenericRaw, Rx>>;

pub type GenericRawRxSocket = DgramSocket<GenericRaw, Rx>;

pub type GenericRawTxSocket = DgramSocket<GenericRaw, Tx>;
