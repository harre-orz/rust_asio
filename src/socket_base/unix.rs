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
