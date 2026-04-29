use super::{ReuseAddr, SockOpt};

pub const MAX_CONNECTIONS: i32 = libc::SOMAXCONN;

/// Possible values which can be passed to the shutdown method.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(i32)]
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = libc::SHUT_RD,

    /// Indicates that the writing portion of this socket should be shut down.
    Write = libc::SHUT_WR,

    /// Shut down both the reading and writing portions of this socket.
    Both = libc::SHUT_RDWR,
}

impl SockOpt for ReuseAddr {
    const KEY: (i32, i32) = (libc::SOL_SOCKET, libc::SO_REUSEADDR);
}
