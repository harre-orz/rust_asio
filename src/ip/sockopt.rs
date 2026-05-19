use crate::ip::{IpProtocol, Tcp};
use crate::socket_base::{GetSockOpt, Protocol, SetSockOpt};

pub struct NoDelay(libc::c_int);

impl NoDelay {
    pub const ON: Self = Self(1);
    pub const OFF: Self = Self(0);

    pub const fn new(on: bool) -> Self {
        Self(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl SetSockOpt<Tcp> for NoDelay {}

impl GetSockOpt<Tcp> for NoDelay {}

pub struct V6Only(libc::c_int);

impl V6Only {
    pub const ON: Self = Self(1);
    pub const OFF: Self = Self(0);

    pub const fn new(on: bool) -> Self {
        Self(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl<P> SetSockOpt<P> for V6Only where P: Protocol<Type = IpProtocol> {}

impl<P> GetSockOpt<P> for V6Only where P: Protocol<Type = IpProtocol> {}

#[cfg(unix)]
mod ffi {
    use super::*;
    use crate::socket_base::SockOpt;

    impl SockOpt<Tcp> for NoDelay {
        const KEY: (i32, i32) = (libc::IPPROTO_TCP, libc::TCP_NODELAY);
    }

    impl<P> SockOpt<P> for V6Only
    where
        P: Protocol<Type = IpProtocol>,
    {
        const KEY: (i32, i32) = (libc::IPPROTO_IPV6, libc::IPV6_V6ONLY);
    }
}
