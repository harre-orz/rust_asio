use std::io;
use {IoObject, Protocol, Endpoint, DgramSocket};
use super::LocalEndpoint;
use ops;
use ops::{AF_LOCAL, SOCK_DGRAM};

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
