mod endpoint;
pub use self::endpoint::{
    Endpoint, EndpointIntoIter, EndpointIter, EndpointRef, Endpoints, Protocol,
};

mod sockopt;
#[cfg(unix)]
pub use self::sockopt::ReusePort;
pub use self::sockopt::{
    Broadcast, DoNotRoute, GetSockOpt, KeepAlive, Linger, RecvBufSize, ReuseAddr, SendBufSize,
    SetSockOpt,
};

#[cfg(doc)]
pub use crate::socket::{MAX_CONNECTIONS, Shutdown, Socket, SocketType};
