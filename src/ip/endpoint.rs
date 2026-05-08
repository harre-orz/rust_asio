use crate::sockaddr::{SockAddrIp, SockAddrWithLen, SockLen};
use crate::socket_base::{
    Endpoint, EndpointIntoIter, EndpointIter, EndpointRef, Endpoints, Protocol,
};
use std::fmt;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Eq, PartialEq, Debug)]
pub enum IpAddrRef<'a> {
    V4(&'a Ipv4Addr),
    V6(&'a Ipv6Addr),
}

impl<'a> IpAddrRef<'a> {
    pub fn clone(&self) -> IpAddr {
        match self {
            &Self::V4(v4) => IpAddr::V4(v4.clone()),
            &Self::V6(v6) => IpAddr::V6(v6.clone()),
        }
    }
}

impl<'a> PartialEq<IpAddr> for IpAddrRef<'a> {
    fn eq(&self, other: &IpAddr) -> bool {
        match (self, other) {
            (&Self::V4(l), IpAddr::V4(r)) => l == r,
            (&Self::V6(l), IpAddr::V6(r)) => l == r,
            _ => false,
        }
    }
}

impl<'a> PartialEq<Ipv4Addr> for IpAddrRef<'a> {
    fn eq(&self, other: &Ipv4Addr) -> bool {
        match self {
            &Self::V4(v4) => v4 == other,
            &Self::V6(_) => false,
        }
    }
}

impl<'a> PartialEq<Ipv6Addr> for IpAddrRef<'a> {
    fn eq(&self, other: &Ipv6Addr) -> bool {
        match self {
            &Self::V4(_) => false,
            &Self::V6(v6) => v6 == other,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct IpProtocol(i32);

impl IpProtocol {
    pub unsafe fn from_raw(ipproto: i32) -> Self {
        Self(ipproto)
    }
}

impl Into<i32> for IpProtocol {
    fn into(self) -> i32 {
        self.0
    }
}

#[cfg(unix)]
mod ffi {
    use super::IpProtocol;

    impl IpProtocol {
        pub const IPPROTO_TCP: Self = Self(libc::IPPROTO_TCP);
        pub const IPPROTO_UDP: Self = Self(libc::IPPROTO_UDP);
        pub const IPPROTO_RAW: Self = Self(libc::IPPROTO_RAW);
        pub const IPPROTO_ICMP: Self = Self(libc::IPPROTO_ICMP);
        pub const IPPROTO_ICMPV6: Self = Self(libc::IPPROTO_ICMPV6);
    }
}

#[cfg(windows)]
mod ffi {
    use super::IpProtocol;
    use windows_sys::Win32::Networking::WinSock;

    impl IpProtocol {
        pub const IPPROTO_TCP: Self = Self(WinSock::IPPROTO_TCP);
        pub const IPPROTO_UDP: Self = Self(WinSock::IPPROTO_UDP);
        pub const IPPROTO_RAW: Self = Self(WinSock::IPPROTO_RAW);
        pub const IPPROTO_ICMP: Self = Self(WinSock::IPPROTO_ICMP);
        pub const IPPROTO_ICMPV6: Self = Self(WinSock::IPPROTO_ICMPV6);
    }
}

/// The internet-protocol endpoint.
#[derive(Copy, Clone)]
pub struct IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol>,
{
    sa: SockAddrIp,
    #[cfg(not(target_os = "macos"))]
    sa_len: SockLen,
    _marker: PhantomData<P>,
}

impl<P> IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol>,
{
    /// Creates from `std::net::IpAddr`.
    pub const fn new(addr: IpAddr, port: u16) -> Self {
        match addr {
            IpAddr::V4(addr) => Self::v4(addr, port),
            IpAddr::V6(addr) => Self::v6(addr, port),
        }
    }

    /// Creates from `std::net::Ipv4Addr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::TcpEndpoint;
    /// use std::net::Ipv4Addr;
    ///
    /// let ep = TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 80);
    /// assert_eq!(ep.as_ipv4_addr().unwrap(), &Ipv4Addr::LOCALHOST);
    /// assert_eq!(ep.port(), 80);
    /// ```
    pub const fn v4(addr: Ipv4Addr, port: u16) -> Self {
        let (sa, sa_len) = SockAddrIp::v4(addr, port).unwrap();
        IpEndpoint {
            sa: sa,
            #[cfg(not(target_os = "macos"))]
            sa_len: sa_len,
            _marker: PhantomData,
        }
    }

    /// Creates from `std::net::Ipv6Addr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::TcpEndpoint;
    /// use std::net::Ipv6Addr;
    ///
    /// let ep = TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 80);
    /// assert_eq!(ep.as_ipv6_addr().unwrap().0, &Ipv6Addr::LOCALHOST);
    /// assert_eq!(ep.port(), 80);
    /// ```
    pub const fn v6(addr: Ipv6Addr, port: u16) -> Self {
        Self::with_scope_id(addr, port, 0)
    }

    /// Creates from `std::net::Ipv6Addr` with scope id.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::ip::TcpEndpoint;
    /// use std::net::Ipv6Addr;
    ///
    /// let ep = TcpEndpoint::with_scope_id(Ipv6Addr::LOCALHOST, 80, 1);
    /// let (ip_addr, scope_id) = ep.as_ipv6_addr().unwrap();
    /// assert_eq!(ip_addr, &Ipv6Addr::LOCALHOST);
    /// assert_eq!(ep.port(), 80);
    /// assert_eq!(scope_id, 1);
    /// ```
    pub const fn with_scope_id(addr: Ipv6Addr, port: u16, scope_id: u32) -> Self {
        let (sa, sa_len) = SockAddrIp::v6(addr, port, scope_id).unwrap();
        IpEndpoint {
            sa: sa,
            #[cfg(not(target_os = "macos"))]
            sa_len: sa_len,
            _marker: PhantomData,
        }
    }

    #[cfg(not(target_os = "macos"))]
    const fn len(&self) -> SockLen {
        self.sa_len as SockLen
    }
    #[cfg(target_os = "macos")]
    const fn len(&self) -> SockLen {
        self.sa.len() as SockLen
    }

    pub const fn is_v4(&self) -> bool {
        self.sa.is_v4()
    }

    pub const fn is_v6(&self) -> bool {
        !self.sa.is_v4()
    }

    pub const fn as_ip_addr(&self) -> IpAddrRef<'_> {
        if self.is_v4() {
            IpAddrRef::V4(unsafe { self.sa.as_ipv4_addr_unchecked() })
        } else {
            IpAddrRef::V6(unsafe { self.sa.as_ipv6_addr_unchecked() })
        }
    }

    pub const fn as_ipv4_addr(&self) -> Option<&Ipv4Addr> {
        if self.sa.is_v4() {
            unsafe { Some(self.as_ipv4_addr_unchecked()) }
        } else {
            None
        }
    }

    pub const unsafe fn as_ipv4_addr_unchecked(&self) -> &Ipv4Addr {
        unsafe { self.sa.as_ipv4_addr_unchecked() }
    }

    pub const fn as_ipv6_addr(&self) -> Option<(&Ipv6Addr, u32)> {
        if !self.sa.is_v4() {
            unsafe {
                Some((
                    self.sa.as_ipv6_addr_unchecked(),
                    self.sa.scope_id_unchecked(),
                ))
            }
        } else {
            None
        }
    }

    pub const unsafe fn as_ipv6_addr_unchecked(&self) -> &Ipv6Addr {
        unsafe { self.sa.as_ipv6_addr_unchecked() }
    }

    pub const unsafe fn scope_id_unchecked(&self) -> u32 {
        unsafe { self.sa.scope_id_unchecked() }
    }

    pub const fn port(&self) -> u16 {
        self.sa.port()
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { self.sa.as_bytes_unchecked(self.len()) }
    }
}

impl<P> Endpoint for IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol>,
{
    type SockAddr = SockAddrIp;

    fn sockaddr_ref(&self) -> &Self::SockAddr {
        &self.sa
    }

    fn sockaddr_len(&self) -> SockLen {
        self.len()
    }

    unsafe fn from_sockaddr(sa_with_len: SockAddrWithLen<Self::SockAddr>) -> Self {
        let (sa, sa_len) = sa_with_len.unwrap();
        IpEndpoint {
            sa: sa,
            #[cfg(not(target_os = "macos"))]
            sa_len: sa_len,
            _marker: PhantomData,
        }
    }
}

impl<P> fmt::Debug for IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", EndpointRef::new(self))
    }
}

impl<'a, P> EndpointRef<'a, IpEndpoint<P>>
where
    P: Protocol<Endpoint = IpEndpoint<P>, Type = IpProtocol>,
{
    pub const fn is_v4(&self) -> bool {
        self.sockaddr_ref().is_v4()
    }

    pub const fn is_v6(&self) -> bool {
        !self.sockaddr_ref().is_v4()
    }

    pub const fn as_ip_addr(&self) -> IpAddrRef<'_> {
        if self.is_v4() {
            IpAddrRef::V4(unsafe { self.sockaddr_ref().as_ipv4_addr_unchecked() })
        } else {
            IpAddrRef::V6(unsafe { self.sockaddr_ref().as_ipv6_addr_unchecked() })
        }
    }

    pub const fn as_ipv4_addr(&self) -> Option<&Ipv4Addr> {
        if self.sockaddr_ref().is_v4() {
            unsafe { Some(self.as_ipv4_addr_unchecked()) }
        } else {
            None
        }
    }

    pub const unsafe fn as_ipv4_addr_unchecked(&self) -> &Ipv4Addr {
        unsafe { self.sockaddr_ref().as_ipv4_addr_unchecked() }
    }

    pub const fn as_ipv6_addr(&self) -> Option<(&Ipv6Addr, u32)> {
        if !self.sockaddr_ref().is_v4() {
            unsafe {
                Some((
                    self.sockaddr_ref().as_ipv6_addr_unchecked(),
                    self.sockaddr_ref().scope_id_unchecked(),
                ))
            }
        } else {
            None
        }
    }

    pub const unsafe fn as_ipv6_addr_unchecked(&self) -> &Ipv6Addr {
        unsafe { self.sockaddr_ref().as_ipv6_addr_unchecked() }
    }

    pub const unsafe fn scope_id_unchecked(&self) -> u32 {
        unsafe { self.sockaddr_ref().scope_id_unchecked() }
    }

    pub const fn port(&self) -> u16 {
        self.sockaddr_ref().port()
    }
}

impl<'a, P> fmt::Debug for EndpointRef<'a, IpEndpoint<P>>
where
    P: Protocol<Endpoint = IpEndpoint<P>, Type = IpProtocol>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_v4() {
            let addr = unsafe { self.sockaddr_ref().as_ipv4_addr_unchecked() };
            write!(
                f,
                "IpEndpoint {{ sockaddr_in {{ addr: {}, port: {} }} }}",
                addr,
                self.sockaddr_ref().port()
            )
        } else {
            let addr = unsafe { self.sockaddr_ref().as_ipv6_addr_unchecked() };
            let scope_id = unsafe { self.sockaddr_ref().scope_id_unchecked() };
            let port = self.sockaddr_ref().port();
            write!(
                f,
                "IpEndpoint {{ sockaddr_in6 {{ addr: {}, port: {}, scope_id: {} }} }}",
                addr, port, scope_id
            )
        }
    }
}

impl<P> From<(IpAddr, u16)> for IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol>,
{
    fn from((addr, port): (IpAddr, u16)) -> Self {
        Self::new(addr, port)
    }
}

impl<P> From<(Ipv4Addr, u16)> for IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol>,
{
    fn from((addr, port): (Ipv4Addr, u16)) -> Self {
        Self::v4(addr, port)
    }
}

impl<P> From<(Ipv6Addr, u16)> for IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol>,
{
    fn from((addr, port): (Ipv6Addr, u16)) -> Self {
        Self::v6(addr, port)
    }
}

impl<'a, P> PartialEq<EndpointRef<'a, Self>> for IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol> + 'a,
{
    fn eq(&self, other: &EndpointRef<'a, Self>) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<P> PartialEq for IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol>,
{
    fn eq(&self, rhs: &Self) -> bool {
        self.as_bytes().eq(rhs.as_bytes())
    }
}

impl<P> Eq for IpEndpoint<P> where P: Protocol<Endpoint = Self, Type = IpProtocol> {}

impl<'a, P> Endpoints<'a, P> for &'a IpEndpoint<P>
where
    P: Protocol<Endpoint = IpEndpoint<P>, Type = IpProtocol> + 'a,
{
    type Iter = EndpointIter<'a, P>;

    fn endpoints(self) -> Self::Iter {
        EndpointIter::new(self)
    }
}

impl<'a, P> Endpoints<'a, P> for IpEndpoint<P>
where
    P: Protocol<Endpoint = Self, Type = IpProtocol> + 'a,
{
    type Iter = EndpointIntoIter<'a, P>;

    fn endpoints(self) -> Self::Iter {
        EndpointIntoIter::new(self)
    }
}
