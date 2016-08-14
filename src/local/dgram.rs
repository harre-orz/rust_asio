use {Protocol, DgramSocket};
use backbone::{AF_LOCAL, SOCK_DGRAM};
use super::{LocalProtocol, LocalEndpoint};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalDgram;

impl Protocol for LocalDgram {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl LocalProtocol for LocalDgram {
}

impl LocalEndpoint<LocalDgram> {
    pub fn protocol(&self) -> LocalDgram {
        LocalDgram
    }
}

pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

pub type LocalDgramSocket = DgramSocket<LocalDgram>;

#[test]
fn test_dgram() {
    assert!(LocalDgram == LocalDgram);
}
