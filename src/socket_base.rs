#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use self::unix::{AddressFamily, MAX_CONNECTIONS, Shutdown, SockAddr, SocketType};

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use self::windows::{AddressFamily, MAX_CONNECTIONS, Shutdown, SockAddr, SocketType};

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
