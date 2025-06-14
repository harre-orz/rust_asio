mod sockaddr;
pub use self::sockaddr::{SockAddr, SockAddrIp, SockAddrStorage, SockAddrUnix};

mod socket;

#[cfg(target_os = "linux")]
pub use self::socket::signalfd;

pub use self::socket::{
    Socket, accept, bind, connect, getpeername, getsockname, listen, read, receive, receive_from,
    reuse_addr, send, send_to, shutdown, socket, socketpair, wait_for_readable, wait_for_writable,
    write,
};

mod signal;

#[cfg(target_os = "linux")]
pub use self::signal::signal_read;

pub use self::signal::{Signal, sigaddset, sigemptyset, sigfillset, sigprocmask};

mod addrinfo;
pub use self::addrinfo::{freeaddrinfo, getaddrinfo};
