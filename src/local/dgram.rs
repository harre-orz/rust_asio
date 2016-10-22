use std::mem;
use {Protocol, Endpoint, DgramSocket};
use libc::{AF_UNIX, SOCK_DGRAM};
use super::{LocalProtocol, LocalEndpoint};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalDgram;

impl Protocol for LocalDgram {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_UNIX
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl LocalProtocol for LocalDgram {
}

impl Endpoint<LocalDgram> for LocalEndpoint<LocalDgram> {
    fn protocol(&self) -> LocalDgram {
        LocalDgram
    }
}

pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

pub type LocalDgramSocket = DgramSocket<LocalDgram>;

#[test]
fn test_dgram() {
    assert!(LocalDgram == LocalDgram);
}
