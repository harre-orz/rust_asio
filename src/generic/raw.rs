//

use super::GenericEndpoint;
use dgram_socket::DgramSocket;
use libc;
use socket_base::Protocol;
use std::mem;

pub struct GenericRaw {
    family: i32,
    protocol: i32,
}

impl GenericRaw {
    pub fn new(family: i32, protocol: i32) -> Self {
        GenericRaw {
            family: family,
            protocol: protocol,
        }
    }
}

impl Protocol for GenericRaw {
    type Endpoint = GenericEndpoint<Self>;
    type Socket = DgramSocket<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_RAW
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        let data: Box<[u8]> = Box::new([0; mem::size_of::<libc::sockaddr_storage>()]);
        GenericEndpoint::new(data)
    }
}

pub type GenericRawEndpoint = GenericEndpoint<GenericRaw>;

pub type GenericRawSocket = DgramSocket<GenericRaw>;
