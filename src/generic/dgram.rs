//

use super::GenericEndpoint;
use dgram_socket::DgramSocket;
use socket_base::Protocol;
use std::mem::{self, MaybeUninit};

#[derive(Clone)]
pub struct GenericDgram {
    family: i32,
    protocol: i32,
}

impl GenericDgram {
    pub fn new(family: i32, protocol: i32) -> Self {
        GenericDgram {
            family: family,
            protocol: protocol,
        }
    }
}

impl Protocol for GenericDgram {
    type Endpoint = GenericEndpoint<Self>;
    type Socket = DgramSocket<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_DGRAM as _
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    fn uninit(&self) -> MaybeUninit<Self::Endpoint> {
        let data: Box<[u8]> = Box::new([0; mem::size_of::<libc::sockaddr_storage>()]);
        MaybeUninit::new(GenericEndpoint::new(data))
    }
}

pub type GenericDgramEndpoint = GenericEndpoint<GenericDgram>;

pub type GenericDgramSocket = DgramSocket<GenericDgram>;
