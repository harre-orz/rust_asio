use ffi::{SOCK_SEQPACKET, sockaddr, socklen_t};
use prelude::{Endpoint, Protocol};
use generic::{GenericEndpoint};
use dgram_socket::DgramSocket;
use socket_builder::SocketBuilder;
use socket_listener::SocketListener;
use socket_base::{Tx, Rx};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GenericSeqPacket {
    family: i32,
    protocol: i32,
    capacity: socklen_t,
}

impl GenericEndpoint<GenericSeqPacket> {
    pub fn protocol(&self) -> GenericSeqPacket {
        GenericSeqPacket {
            family: unsafe { &*self.as_ptr() }.sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }
}

impl Protocol for GenericSeqPacket {
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_SEQPACKET
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        GenericEndpoint::default(self.capacity, self.protocol)
    }
}

impl Endpoint<GenericSeqPacket> for GenericEndpoint<GenericSeqPacket> {
    fn protocol(&self) -> GenericSeqPacket {
         GenericSeqPacket {
            family: unsafe { &*self.as_ptr() }.sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }

    fn as_ptr(&self) -> *const sockaddr {
        self.sa.sa.as_ptr() as *const _
    }

    fn as_mut_ptr(&mut self) -> *mut sockaddr {
        self.sa.sa.as_mut_ptr() as *mut _
    }

    fn capacity(&self) -> socklen_t {
        self.sa.capacity() as socklen_t
    }

    fn size(&self) -> socklen_t {
        self.sa.size() as socklen_t
    }

    unsafe fn resize(&mut self, size: socklen_t) {
        debug_assert!(size <= self.capacity());
        self.sa.resize(size as u8)
    }
}

pub type GenericSeqPacketEndpoint = GenericEndpoint<GenericSeqPacket>;

pub type GenericSeqPacketBuilder = SocketBuilder<GenericSeqPacket, DgramSocket<GenericSeqPacket, Tx>, DgramSocket<GenericSeqPacket, Rx>>;

pub type GenericSeqPacketListener = SocketListener<GenericSeqPacket, DgramSocket<GenericSeqPacket, Tx>, DgramSocket<GenericSeqPacket, Rx>>;

pub type GenericSeqPacketRxSocket = DgramSocket<GenericSeqPacket, Rx>;

pub type GenericSeqPacketTxSocket = DgramSocket<GenericSeqPacket, Tx>;
