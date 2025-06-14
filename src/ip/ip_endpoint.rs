use crate::sockaddr::ffi::SockAddrIp;
use crate::socket_base::{AddressFamily, Endpoint, IntoProtocolType, Protocol};
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct IpProtocol(i32);

#[cfg(unix)]
mod ffi {
    use super::*;

    impl IpProtocol {
        pub const TCP: Self = Self(libc::IPPROTO_TCP);
        pub const UDP: Self = Self(libc::IPPROTO_UDP);
        pub const RAW: Self = Self(libc::IPPROTO_RAW);
        pub const ICMP: Self = Self(libc::IPPROTO_ICMP);
        pub const ICMPV6: Self = Self(libc::IPPROTO_ICMPV6);
    }
}

#[cfg(windows)]
mod ffi {
    use super::*;
    use windows_sys::Win32::Networking::WinSock;

    impl IpProtocol {
        pub const TCP: Self = Self(WinSock::IPPROTO_TCP);
        pub const UDP: Self = Self(WinSock::IPPROTO_UDP);
        pub const RAW: Self = Self(WinSock::IPPROTO_RAW);
        pub const ICMP: Self = Self(WinSock::IPPROTO_ICMP);
        pub const ICMPV6: Self = Self(WinSock::IPPROTO_ICMPV6);
    }
}

impl Into<i32> for IpProtocol {
    fn into(self) -> i32 {
        self.0 as i32
    }
}

impl IntoProtocolType for IpProtocol {}

#[derive(Copy, Clone, Debug)]
pub struct IpEndpoint<P> {
    inner: SockAddrIp,
    _marker: PhantomData<P>,
}

impl<P> IpEndpoint<P> {
    pub const fn new(addr: IpAddr, port: u16) -> Self {
        match addr {
            IpAddr::V4(addr) => Self::v4(addr, port),
            IpAddr::V6(addr) => Self::v6(addr, port, 0),
        }
    }

    pub const fn v4(addr: Ipv4Addr, port: u16) -> Self {
        IpEndpoint {
            inner: SockAddrIp::v4(addr, port),
            _marker: PhantomData,
        }
    }

    pub const fn v6(addr: Ipv6Addr, port: u16, scope_id: u32) -> Self {
        IpEndpoint {
            inner: SockAddrIp::v6(addr, port, scope_id),
            _marker: PhantomData,
        }
    }

    pub const fn family_type(&self) -> AddressFamily {
        self.inner.family_type()
    }

    pub const fn is_v4(&self) -> bool {
        self.family_type().get() == AddressFamily::INET.get()
    }

    pub const fn is_v6(&self) -> bool {
        self.family_type().get() == AddressFamily::INET6.get()
    }

    pub fn addr(&self) -> IpAddr {
        match self.family_type() {
            AddressFamily::INET => IpAddr::V4(unsafe { self.as_ipv4_addr() }.clone()),
            AddressFamily::INET6 => IpAddr::V6(unsafe { self.as_ipv6_addr() }.clone()),
            _ => unreachable!(),
        }
    }

    pub const unsafe fn as_ipv4_addr(&self) -> &Ipv4Addr {
        unsafe {
            self.inner.as_ipv4_addr()
        }
    }

    pub const unsafe fn as_ipv6_addr(&self) -> &Ipv6Addr {
        unsafe {
            self.inner.as_ipv6_addr()
        }
    }

    pub const fn port(&self) -> u16 {
        self.inner.port()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

impl<P> Endpoint for IpEndpoint<P>
where
    P: Protocol,
{
    type SockAddr = SockAddrIp;

    fn new(sa: Self::SockAddr) -> Self {
        Self {
            inner: sa,
            _marker: PhantomData,
        }
    }

    fn sockaddr(&self) -> &Self::SockAddr {
        &self.inner
    }
}

impl<P> From<(IpAddr, u16)> for IpEndpoint<P> {
    fn from((addr, port): (IpAddr, u16)) -> Self {
        Self::new(addr, port)
    }
}

impl<P> From<(Ipv4Addr, u16)> for IpEndpoint<P> {
    fn from((addr, port): (Ipv4Addr, u16)) -> Self {
        Self::v4(addr, port)
    }
}

impl<P> From<(Ipv6Addr, u16)> for IpEndpoint<P> {
    fn from((addr, port): (Ipv6Addr, u16)) -> Self {
        Self::v6(addr, port, 0)
    }
}

// impl<P> fmt::Debug for IpEndpoint<P> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self.addr() {
//             IpAddr::V4(addr) => write!(
//                 f,
//                 "IpEndpoint {{ addr: {}, port: {} }}",
//                 addr,
//                 self.port()
//             ),
//             IpAddr::V6(addr) => write!(
//                 f,
//                 "IpEndpoint {{ addr: {}, port: {} }}",
//                 addr,
//                 self.port()
//             ),
//         }
//     }
// }

impl<P> PartialEq for IpEndpoint<P> {
    fn eq(&self, rhs: &Self) -> bool {
        self.inner.as_bytes().eq(rhs.inner.as_bytes())
    }
}

impl<P> Eq for IpEndpoint<P> {}
