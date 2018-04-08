use ffi::{sockaddr, socklen_t, SOCK_RAW};
use core::{Endpoint, Protocol};
use dgram_socket::DgramSocket;
use generic::GenericEndpoint;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GenericRaw {
    family: i32,
    protocol: i32,
    capacity: socklen_t,
}

impl Protocol for GenericRaw {
    type Endpoint = GenericEndpoint<Self>;

    type Socket = DgramSocket<GenericRaw>;

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

impl Endpoint<GenericRaw> for GenericEndpoint<GenericRaw> {
    fn protocol(&self) -> GenericRaw {
        GenericRaw {
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

pub type GenericRawEndpoint = GenericEndpoint<GenericRaw>;

pub type GenericRawSocket = DgramSocket<GenericRaw>;
