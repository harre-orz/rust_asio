use super::{ReuseAddr, SockOpt};
use windows_sys::Win32::Networking::WinSock;

pub const MAX_CONNECTIONS: u32 = WinSock::SOMAXCONN;

/// Possible values which can be passed to the shutdown method.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(i32)]
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = WinSock::SD_RECEIVE,

    /// Indicates that the writing portion of this socket should be shut down.
    Write = WinSock::SD_SEND,

    /// Shut down both the reading and writing portions of this socket.
    Both = WinSock::SD_BOTH,
}

impl SockOpt for ReuseAddr {
    const KEY: (i32, i32) = (WinSock::SOL_SOCKET, WinSock::SO_REUSEADDR);
}
