//

use super::LocalEndpoint;
use dgram_socket::DgramSocket;
use libc;
use socket_base::Protocol;
use std::mem::MaybeUninit;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalDgram;

impl LocalEndpoint<LocalDgram> {
    pub fn protocol(&self) -> LocalDgram {
        LocalDgram
    }
}

impl Protocol for LocalDgram {
    type Endpoint = LocalEndpoint<Self>;
    type Socket = DgramSocket<Self>;

    fn family_type(&self) -> i32 {
        libc::AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_STREAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    fn uninit(&self) -> MaybeUninit<Self::Endpoint> {
        MaybeUninit::uninit()
    }
}

pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

pub type LocalDgramSocket = DgramSocket<LocalDgram>;
