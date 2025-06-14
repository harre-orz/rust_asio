#[cfg(unix)]
mod ffi {
    use std::mem::MaybeUninit;
    use std::ptr;

    pub const MAX_CONNECTIONS: i32 = libc::SOMAXCONN;

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
    pub struct AddressFamily(libc::sa_family_t);

    impl AddressFamily {
        pub const UNIX: Self = Self(libc::AF_UNIX as libc::sa_family_t);
        pub const INET: Self = Self(libc::AF_INET as libc::sa_family_t);
        pub const INET6: Self = Self(libc::AF_INET6 as libc::sa_family_t);
        pub const UNSPEC: Self = Self(libc::AF_UNSPEC as libc::sa_family_t);

        pub(crate) const unsafe fn new_unchecked(sa_family: libc::sa_family_t) -> Self {
            Self(sa_family)
        }

        pub(crate) const fn get(&self) -> libc::sa_family_t {
            self.0
        }
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
    pub struct SocketType(pub(super) u16);

    impl SocketType {
        pub const STREAM: Self = Self(libc::SOCK_STREAM as u16);
        pub const DGRAM: Self = Self(libc::SOCK_DGRAM as u16);
        pub const SEQPACKET: Self = Self(libc::SOCK_SEQPACKET as u16);
        pub const RAW: Self = Self(libc::SOCK_RAW as u16);

        pub(crate) const fn get(&self) -> u16 {
            self.0
        }
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

    pub trait SockAddr: Sized {
        const MAX_SIZE: libc::socklen_t = size_of::<Self>() as libc::socklen_t;

        unsafe fn init(sa: MaybeUninit<Self>, len: libc::socklen_t) -> Self;

        #[cfg(target_os = "linux")]
        fn len(&self) -> libc::socklen_t;

        #[cfg(target_os = "macos")]
        fn len(&self) -> libc::socklen_t {
            unsafe { &*self.as_ptr() }.sa_len as libc::socklen_t
        }

        fn as_ptr(&self) -> *const libc::sockaddr {
            ptr::from_ref(self).cast()
        }
    }
}

#[cfg(windows)]
mod ffi {
    use super::*;
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
}

pub use self::ffi::{AddressFamily, MAX_CONNECTIONS, Shutdown, SockAddr, SocketType};

impl Into<i32> for AddressFamily {
    fn into(self) -> i32 {
        self.get() as i32
    }
}

impl Into<i32> for SocketType {
    fn into(self) -> i32 {
        self.get() as i32
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
