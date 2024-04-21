use std::mem::MaybeUninit;

pub type SockaddrType = *const libc::sockaddr;
pub type SocklenType = libc::socklen_t;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct AddressFamily(pub(crate) u16);

impl AddressFamily {
    pub const UNIX: Self = Self(libc::AF_UNIX as u16);
    pub const INET: Self = Self(libc::AF_INET as u16);
    pub const INET6: Self = Self(libc::AF_INET6 as u16);
    pub const UNSPEC: Self = Self(libc::AF_UNSPEC as u16);
}

impl Into<i32> for AddressFamily {
    fn into(self) -> i32 {
        self.0 as i32
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct SocketType(pub(crate) u16);

impl SocketType {
    pub const STREAM: Self = Self(libc::SOCK_STREAM as u16);
    pub const DGRAM: Self = Self(libc::SOCK_DGRAM as u16);
    pub const SEQPACKET: Self = Self(libc::SOCK_SEQPACKET as u16);
    pub const RAW: Self = Self(libc::SOCK_RAW as u16);
}

impl Into<i32> for SocketType {
    fn into(self) -> i32 {
        self.0 as i32
    }
}

pub trait IntoProtocolType: Into<i32> {}

pub trait Endpoint: Sized {
    const SIZE: SocklenType;

    fn as_ptr(&self) -> SockaddrType;
    fn len(&self) -> SocklenType;

    unsafe fn init(sa: MaybeUninit<Self>, salen: SocklenType) -> Self;
}

pub trait Protocol: Copy {
    type Type: IntoProtocolType;
    type Endpoint: Endpoint;

    fn family_type(self) -> AddressFamily;
    fn socket_type(self) -> SocketType;
    fn protocol_type(self) -> Self::Type;
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

impl Into<i32> for Shutdown {
    fn into(self) -> i32 {
        self as i32
    }
}
