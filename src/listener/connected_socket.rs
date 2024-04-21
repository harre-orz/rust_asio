use crate::IoContext;
use std::os::fd::OwnedFd;

pub struct ConnectedSocket {
    pub(crate) ctx: IoContext,
    pub(crate) soc: OwnedFd,
}

pub trait IntoConnectedSocket {
    type Socket;

    fn into_connected_socket(&self, conn: ConnectedSocket) -> Self::Socket;
}
