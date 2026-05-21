mod endpoint;
pub use self::endpoint::{
    Endpoint, EndpointIntoIter, EndpointIter, EndpointRef, Endpoints, Protocol,
};

mod sockopt;
pub use self::sockopt::{
    Broadcast, DoNotRoute, GetSockOpt, KeepAlive, Linger, RecvBufSize, ReuseAddr, ReusePort,
    SendBufSize, SetSockOpt,
};

#[cfg(doc)]
pub use crate::socket::{MAX_CONNECTIONS, Shutdown, Socket, SocketType};
