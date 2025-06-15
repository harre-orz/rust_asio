use std::mem::MaybeUninit;
use std::ptr;
use windows_sys::Win32::Networking::WinSock;

pub const MAX_CONNECTIONS: i32 = WinSock::SOMAXCONN as i32;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct AddressFamily(WinSock::ADDRESS_FAMILY);

impl AddressFamily {
    pub const UNIX: Self = Self(WinSock::AF_UNIX);
    pub const INET: Self = Self(WinSock::AF_INET);
    pub const INET6: Self = Self(WinSock::AF_INET6);
    pub const UNSPEC: Self = Self(WinSock::AF_UNSPEC);

    pub(crate) const unsafe fn new_unchecked(family: WinSock::ADDRESS_FAMILY) -> Self {
        AddressFamily(family)
    }

    pub(crate) const fn get(&self) -> WinSock::ADDRESS_FAMILY {
        self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct SocketType(WinSock::WINSOCK_SOCKET_TYPE);

impl SocketType {
    pub const STREAM: Self = SocketType(WinSock::SOCK_STREAM);
    pub const DGRAM: Self = SocketType(WinSock::SOCK_DGRAM);
    pub const SEQPACKET: Self = SocketType(WinSock::SOCK_SEQPACKET);
    pub const RAW: Self = SocketType(WinSock::SOCK_RAW);

    pub(crate) const fn get(&self) -> WinSock::WINSOCK_SOCKET_TYPE {
        self.0
    }
}

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

pub trait SockAddr: Sized {
    const MAX_SIZE: WinSock::socklen_t = size_of::<Self>() as WinSock::socklen_t;

    unsafe fn init(sa: MaybeUninit<Self>, len: WinSock::socklen_t) -> Self;

    fn len(&self) -> WinSock::socklen_t;

    fn as_ptr(&self) -> *const WinSock::SOCKADDR {
        ptr::from_ref(self).cast()
    }
}
