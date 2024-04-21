use std::os::fd::OwnedFd;

pub struct ConnectedSocket(pub(crate) OwnedFd);

pub trait IntoConnectedSocket {
    type Socket;

    fn into_connected_socket(&self, soc: ConnectedSocket) -> Self::Socket;
}
