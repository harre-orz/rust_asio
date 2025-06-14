
#[cfg(unix)]
mod ffi {
    use super::*;

    impl SocketType {
        pub const STREAM: Self = Self(libc::SOCK_STREAM);
        pub const DGRAM: Self = Self(libc::SOCK_DGRAM);
        pub const SEQPACKET: Self = Self(libc::SOCK_SEQPACKET);
        pub const RAW: Self = Self(libc::SOCK_RAW);
    }

    /// Possible values which can be passed to the shutdown method.
    #[repr(i32)]
    pub enum Shutdown {
        /// Indicates that the reading portion of this socket should be shut down.
        Read = libc::SHUT_RD,

        /// Indicates that the writing portion of this socket should be shut down.
        Write = libc::SHUT_WR,

        /// Shut down both the reading and writing portions of this socket.
        Both = libc::SHUT_RDWR,
    }

    pub const MAX_CONNECTION: i32 = libc::SOMAXCONN;
}

#[cfg(windows)]
mod ffi {
    use super::*;
    use windows_sys::Win32::Networking::WinSock;

    /// Possible values which can be passed to the shutdown method.
    #[repr(i32)]
    pub enum Shutdown {
        /// Indicates that the reading portion of this socket should be shut down.
        Read = WinSock::SD_RECEIVE,

        /// Indicates that the writing portion of this socket should be shut down.
        Write = WinSock::SD_SEND,

        /// Shut down both the reading and writing portions of this socket.
        Both = WinSock::SD_BOTH,
    }

    pub const MAX_CONNECTION: i32 = WinSock::SOMAXCONN as i32;

    impl SocketType {
        pub const STREAM: Self = SocketType(WinSock::SOCK_STREAM as i32);
        pub const DGRAM: Self = SocketType(WinSock::SOCK_DGRAM as i32);
        pub const SEQPACKET: Self = SocketType(WinSock::SOCK_SEQPACKET as i32);
        pub const RAW: Self = SocketType(WinSock::SOCK_RAW as i32);
    }
}

pub use crate::sockaddr::ffi::{AddressFamily, SockAddr};
pub use ffi::{Shutdown, MAX_CONNECTION};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct SocketType(pub(super) i32);

impl Into<i32> for AddressFamily {
    fn into(self) -> i32 {
        self.0 as i32
    }
}

impl Into<i32> for SocketType {
    fn into(self) -> i32 {
        self.0
    }
}

impl Into<i32> for Shutdown {
    fn into(self) -> i32 {
        self as i32
    }
}

pub trait IntoProtocolType: Copy + Into<i32> {}

pub trait Endpoint {
    type SockAddr: SockAddr;

    fn new(sa: Self::SockAddr) -> Self;
    fn sockaddr(&self) -> &Self::SockAddr;
}

pub trait Protocol: Copy {
    type Type: IntoProtocolType;
    type Endpoint: Endpoint;

    fn family_type(self) -> AddressFamily;
    fn socket_type(self) -> SocketType;
    fn protocol_type(self) -> Self::Type;
}
