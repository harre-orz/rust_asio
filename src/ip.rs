use crate::IoContext;
use crate::dgram_socket::DgramSocket;
use crate::error::{OsError, ResolverError};
use crate::ffi::sockaddr::SockAddrIp;
use crate::ffi::socket::Socket;
use crate::ip::ffi::AddrInfo;
use crate::socket_base::{AddressFamily, Endpoint, IntoProtocolType, Protocol, SocketType};
use crate::socket_listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::stream_socket::{AsyncStreamSocket, StreamSocket};
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct IpProtocol(i32);

#[cfg(unix)]
mod ffi {
    use super::{IpProtocol, ResolverQuery};
    use crate::error::ResolverError;
    use crate::ffi::sockaddr::SockAddrIp;
    use crate::socket_base::Protocol;
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::ptr;

    impl IpProtocol {
        pub const TCP: Self = Self(libc::IPPROTO_TCP);
        pub const UDP: Self = Self(libc::IPPROTO_UDP);
        pub const RAW: Self = Self(libc::IPPROTO_RAW);
        pub const ICMP: Self = Self(libc::IPPROTO_ICMP);
        pub const ICMPV6: Self = Self(libc::IPPROTO_ICMPV6);
    }

    impl ResolverQuery {
        pub(super) fn from_rr<T, U>(host: T, port: U) -> Self
        where
            T: AsRef<str>,
            U: AsRef<str>,
        {
            Self {
                node: CString::new(host.as_ref()).unwrap(),
                serv: CString::new(port.as_ref()).unwrap(),
                flags: 0,
            }
        }

        pub(super) fn from_rt<T, U>(host: T, port: U) -> Self
        where
            T: AsRef<str>,
            U: ToString,
        {
            Self {
                node: CString::new(host.as_ref()).unwrap(),
                serv: CString::new(port.to_string()).unwrap(),
                flags: libc::AI_NUMERICSERV,
            }
        }

        pub(super) fn from_tt<T, U>(host: T, port: U) -> Self
        where
            T: ToString,
            U: ToString,
        {
            Self {
                node: CString::new(host.to_string()).unwrap(),
                serv: CString::new(port.to_string()).unwrap(),
                flags: libc::AI_NUMERICHOST | libc::AI_NUMERICSERV,
            }
        }
    }

    pub struct AddrInfo<'a> {
        res: &'a mut libc::addrinfo,
        ai: *mut libc::addrinfo,
    }

    impl<'a> Drop for AddrInfo<'a> {
        fn drop(&mut self) {
            unsafe { libc::freeaddrinfo(self.res) }
        }
    }

    impl<'a> AddrInfo<'a> {
        pub fn new<P>(pro: P, query: ResolverQuery) -> Result<Self, ResolverError>
        where
            P: Protocol,
        {
            let node = if query.node.is_empty() {
                ptr::null()
            } else {
                query.node.as_ptr()
            };
            let serv = if query.serv.is_empty() {
                ptr::null()
            } else {
                query.serv.as_ptr()
            };
            let hints = libc::addrinfo {
                ai_flags: query.flags,
                ai_family: pro.family_type().into(),
                ai_socktype: pro.socket_type().into(),
                ai_protocol: pro.protocol_type().into(),
                ai_addrlen: 0,
                ai_addr: ptr::null_mut(),
                ai_canonname: ptr::null_mut(),
                ai_next: ptr::null_mut(),
            };
            let mut res = MaybeUninit::<*mut libc::addrinfo>::uninit();
            unsafe {
                match libc::getaddrinfo(node, serv, &hints, res.as_mut_ptr()) {
                    0 => {
                        let res = res.assume_init();
                        Ok(AddrInfo {
                            res: &mut *res,
                            ai: res,
                        })
                    }
                    err => Err(ResolverError::from_raw(err)),
                }
            }
        }

        pub fn next(&mut self) -> Option<&SockAddrIp> {
            if self.ai.is_null() {
                None
            } else {
                let ai = unsafe { &*self.ai };
                let sa = ai.ai_addr as *const SockAddrIp;
                self.ai = ai.ai_next;
                Some(unsafe { &*sa })
            }
        }
    }
}

#[cfg(windows)]
mod ffi {
    use super::IpProtocol;
    use super::{Protocol, ResolverError, ResolverQuery};
    use crate::ffi::sockaddr::SockAddrIp;
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::ptr;
    use windows_sys::Win32::Networking::WinSock;

    impl IpProtocol {
        pub const TCP: Self = Self(WinSock::IPPROTO_TCP);
        pub const UDP: Self = Self(WinSock::IPPROTO_UDP);
        pub const RAW: Self = Self(WinSock::IPPROTO_RAW);
        pub const ICMP: Self = Self(WinSock::IPPROTO_ICMP);
        pub const ICMPV6: Self = Self(WinSock::IPPROTO_ICMPV6);
    }

    impl ResolverQuery {
        pub(super) fn from_rr<T, U>(host: T, port: U) -> Self
        where
            T: AsRef<str>,
            U: AsRef<str>,
        {
            Self {
                node: CString::new(host.as_ref()).unwrap(),
                serv: CString::new(port.as_ref()).unwrap(),
                flags: 0,
            }
        }

        pub(super) fn from_rt<T, U>(host: T, port: U) -> Self
        where
            T: AsRef<str>,
            U: ToString,
        {
            Self {
                node: CString::new(host.as_ref()).unwrap(),
                serv: CString::new(port.to_string()).unwrap(),
                flags: WinSock::AI_NUMERICSERV as i32,
            }
        }

        pub(super) fn from_tt<T, U>(host: T, port: U) -> Self
        where
            T: ToString,
            U: ToString,
        {
            Self {
                node: CString::new(host.to_string()).unwrap(),
                serv: CString::new(port.to_string()).unwrap(),
                flags: (WinSock::AI_NUMERICHOST | WinSock::AI_NUMERICSERV) as i32,
            }
        }
    }

    pub struct AddrInfo<'a> {
        res: &'a WinSock::ADDRINFOA,
        ai: *mut WinSock::ADDRINFOA,
    }

    impl<'a> Drop for AddrInfo<'a> {
        fn drop(&mut self) {
            unsafe { WinSock::freeaddrinfo(self.res) }
        }
    }

    impl<'a> AddrInfo<'a> {
        pub fn new<P>(pro: P, query: ResolverQuery) -> Result<Self, ResolverError>
        where
            P: Protocol,
        {
            let node = if query.node.is_empty() {
                ptr::null()
            } else {
                query.node.as_ptr().cast()
            };
            let serv = if query.serv.is_empty() {
                ptr::null()
            } else {
                query.serv.as_ptr().cast()
            };
            let hints = WinSock::ADDRINFOA {
                ai_flags: query.flags,
                ai_family: pro.family_type().into(),
                ai_socktype: pro.socket_type().into(),
                ai_protocol: pro.protocol_type().into(),
                ai_addrlen: 0,
                ai_addr: ptr::null_mut(),
                ai_canonname: ptr::null_mut(),
                ai_next: ptr::null_mut(),
            };
            let mut res = MaybeUninit::<*mut WinSock::ADDRINFOA>::uninit();
            unsafe {
                match WinSock::getaddrinfo(node, serv, &hints, res.as_mut_ptr()) {
                    0 => {
                        let res = res.assume_init();
                        Ok(AddrInfo {
                            res: unsafe { &*res },
                            ai: res,
                        })
                    }
                    err => Err(ResolverError::from_raw(err)),
                }
            }
        }

        pub fn next(&mut self) -> Option<&'a SockAddrIp> {
            if self.ai.is_null() {
                None
            } else {
                let ai = unsafe { &*self.ai };
                let sa = ai.ai_addr as *const SockAddrIp;
                self.ai = ai.ai_next;
                Some(unsafe { &*sa })
            }
        }
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
        unsafe { self.inner.as_ipv4_addr() }
    }

    pub const unsafe fn as_ipv6_addr(&self) -> &Ipv6Addr {
        unsafe { self.inner.as_ipv6_addr() }
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

pub struct ResolverQuery {
    node: CString,
    serv: CString,
    flags: i32,
}

impl From<(&str, &str)> for ResolverQuery {
    fn from((host, port): (&str, &str)) -> Self {
        Self::from_rr(host, port)
    }
}

impl From<(&String, &str)> for ResolverQuery {
    fn from((host, port): (&String, &str)) -> Self {
        Self::from_rr(host, port)
    }
}

impl From<(String, &str)> for ResolverQuery {
    fn from((host, port): (String, &str)) -> Self {
        Self::from_rr(host, port)
    }
}

impl From<(&str, u16)> for ResolverQuery {
    fn from((host, port): (&str, u16)) -> Self {
        Self::from_rt(host, port)
    }
}

impl From<(IpAddr, u16)> for ResolverQuery {
    fn from((host, port): (IpAddr, u16)) -> Self {
        Self::from_tt(host, port)
    }
}

impl From<(Ipv4Addr, u16)> for ResolverQuery {
    fn from((host, port): (Ipv4Addr, u16)) -> Self {
        Self::from_tt(host, port)
    }
}

impl From<(Ipv6Addr, u16)> for ResolverQuery {
    fn from((host, port): (Ipv6Addr, u16)) -> Self {
        Self::from_tt(host, port)
    }
}

pub struct ResolverIter<'a, P> {
    ai: AddrInfo<'a>,
    _ctx: IoContext,
    _marker: PhantomData<P>,
}

impl<'a, P> Iterator for ResolverIter<'a, P>
where
    P: Protocol + 'a,
{
    type Item = &'a IpEndpoint<P>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ai.next().map(|sa| unsafe { mem::transmute(sa) })
    }
}

unsafe impl<'a, P> Send for ResolverIter<'a, P> {}

pub struct Resolver<P> {
    ctx: IoContext,
    pro: P,
}

impl<P> Resolver<P>
where
    P: Protocol,
{
    pub(crate) fn new_priv(ctx: &IoContext, pro: P) -> Self {
        Self {
            ctx: ctx.clone(),
            pro: pro,
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn resolve<Q>(&self, query: Q) -> Result<ResolverIter<P>, ResolverError>
    where
        Q: Into<ResolverQuery>,
    {
        AddrInfo::new(self.pro, query.into()).map(|ai| ResolverIter {
            ai: ai,
            _ctx: self.ctx.clone(),
            _marker: PhantomData,
        })
    }
}

/// The Transmission Control Protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Tcp(AddressFamily);

impl Tcp {
    /// Represents a TCP for IPv4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Tcp, TcpEndpoint};
    /// use std::net::Ipv4Addr;
    ///
    ///
    /// let ep = TcpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 0);
    /// assert_eq!(Tcp::V4, ep.protocol());
    /// ```
    pub const V4: Self = Self(AddressFamily::INET);

    /// Represents a TCP for IPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Tcp, TcpEndpoint};
    /// use std::net::Ipv6Addr;
    ///
    ///
    /// let ep = TcpEndpoint::v6(Ipv6Addr::UNSPECIFIED, 0, 0);
    /// assert_eq!(Tcp::V6, ep.protocol());
    /// ```
    pub const V6: Self = Self(AddressFamily::INET6);
}

impl Protocol for Tcp {
    type Type = IpProtocol;
    type Endpoint = IpEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::STREAM
    }

    fn protocol_type(self) -> Self::Type {
        IpProtocol::TCP
    }
}

impl IpEndpoint<Tcp> {
    pub const fn protocol(&self) -> Tcp {
        Tcp(self.family_type())
    }
}

impl ConnectedSocket for SocketListener<Tcp> {
    type Socket = StreamSocket<Tcp>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl ConnectedSocket for AsyncSocketListener<Tcp> {
    type Socket = AsyncStreamSocket<Tcp>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

impl Resolver<Tcp> {
    /// The performs name resolution for TCP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{TcpResolver, TcpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::{Ipv4Addr, Ipv6Addr};
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// for ep in TcpResolver::new(ctx).resolve(("localhost", "http")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, &TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 80));
    ///     }
    ///     if !ep.is_v4() {
    ///         assert_eq!(ep, &TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 80, 0));
    ///     }
    /// }
    /// ```
    pub fn new(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp(AddressFamily::UNSPEC))
    }

    /// The performs name resolution for TCP with IPv4 only.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{TcpResolver, TcpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    ///
    /// for ep in TcpResolver::v4(ctx).resolve(("localhost", "http")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, &TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 80));
    ///     }
    ///     if !ep.is_v4() {
    ///         panic!("{:?}", ep);
    ///     }
    /// }
    /// ```
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp::V4)
    }

    /// The performs name resolution for TCP with IPv6 only.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{TcpResolver, TcpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv6Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// if let Ok(it) = TcpResolver::v6(ctx).resolve(("localhost", "http")) {
    ///     for ep in it {
    ///         if !ep.is_v6() {
    ///            panic!("{:?}", ep);
    ///         }
    ///         if !ep.is_v4() {
    ///             assert_eq!(ep, &TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 80, 0));
    ///         }
    ///     }
    /// }
    /// ```
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp::V6)
    }

    pub fn connect<Q>(
        &self,
        query: Q,
    ) -> Result<(StreamSocket<Tcp>, IpEndpoint<Tcp>), ResolverError>
    where
        Q: Into<ResolverQuery>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in self.resolve(query)? {
            match StreamSocket::new(self.as_ctx(), ep.protocol()) {
                Ok(soc) => match soc.connect(&ep) {
                    Ok(soc) => return Ok((soc, ep.clone())),
                    Err(err_) => err = err_,
                },
                Err(err_) => {
                    err = err_;
                    break;
                }
            }
        }
        Err(ResolverError::from_os_err(err))
    }

    pub async fn async_connect<Q>(
        &self,
        query: Q,
    ) -> Result<(AsyncStreamSocket<Tcp>, IpEndpoint<Tcp>), ResolverError>
    where
        Q: Into<ResolverQuery>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in self.resolve(query)? {
            match StreamSocket::new(self.as_ctx(), ep.protocol()) {
                Ok(soc) => match soc.async_connect(&ep).await {
                    Ok(soc) => return Ok((soc, ep.clone())),
                    Err(err_) => err = err_,
                },
                Err(err_) => {
                    err = err_;
                    break;
                }
            }
        }
        Err(ResolverError::from_os_err(err))
    }
}

/// The TCP endpoint type.
pub type TcpEndpoint = IpEndpoint<Tcp>;

/// The TCP socket type.
pub type TcpSocket = StreamSocket<Tcp>;

/// The TCP resolver type.
pub type TcpResolver = Resolver<Tcp>;

/// The TCP listener type.
pub type TcpListener = SocketListener<Tcp>;

/// The User Datagram Protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Udp(AddressFamily);

impl Udp {
    /// Represents a UDP for IPv4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Udp, UdpEndpoint};
    /// use std::net::Ipv4Addr;
    ///
    ///
    /// let ep = UdpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 0);
    /// assert_eq!(Udp::V4, ep.protocol());
    /// ```
    pub const V4: Self = Self(AddressFamily::INET);

    /// Represents a UDP for IPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Udp, UdpEndpoint};
    /// use std::net::Ipv6Addr;
    ///
    ///
    /// let ep = UdpEndpoint::v6(Ipv6Addr::UNSPECIFIED, 0, 0);
    /// assert_eq!(Udp::V6, ep.protocol());
    /// ```
    pub const V6: Self = Self(AddressFamily::INET6);
}

impl Protocol for Udp {
    type Type = IpProtocol;
    type Endpoint = IpEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::DGRAM
    }

    fn protocol_type(self) -> Self::Type {
        IpProtocol::UDP
    }
}

impl IpEndpoint<Udp> {
    pub const fn protocol(&self) -> Udp {
        Udp(self.family_type())
    }
}

impl Resolver<Udp> {
    /// The performs name resolution for UDP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{UdpResolver, UdpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::{Ipv4Addr, Ipv6Addr};
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// for ep in UdpResolver::new(ctx).resolve(("localhost", "12345")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, &UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
    ///     }
    ///     if !ep.is_v4() {
    ///         assert_eq!(ep, &UdpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345, 0));
    ///     }
    /// }
    /// ```
    pub fn new(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp(AddressFamily::UNSPEC))
    }

    /// The performs name resolution for UDP with IPv4 only.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{UdpResolver, UdpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// for ep in UdpResolver::v4(ctx).resolve(("localhost", "12345")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, &UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
    ///     }
    ///     if !ep.is_v4() {
    ///         panic!("{:?}", ep);
    ///     }
    /// }
    /// ```
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp::V4)
    }

    /// The performs name resolution for UDP with IPv6 only.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{UdpResolver, UdpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv6Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// if let Ok(it) = UdpResolver::v6(ctx).resolve(("localhost", "12345")) {
    ///     for ep in it {
    ///         if !ep.is_v6() {
    ///            panic!("{:?}", ep);
    ///         }
    ///         if !ep.is_v4() {
    ///             assert_eq!(ep, &UdpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345, 0));
    ///         }
    ///     }
    /// }
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp::V6)
    }

    pub fn connect<Q>(&self, query: Q) -> Result<(DgramSocket<Udp>, IpEndpoint<Udp>), ResolverError>
    where
        Q: Into<ResolverQuery>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in self.resolve(query)? {
            match DgramSocket::new(self.as_ctx(), ep.protocol()) {
                Ok(soc) => match soc.connect(&ep) {
                    Ok(_) => return Ok((soc, ep.clone())),
                    Err(err_) => err = err_,
                },
                Err(err_) => {
                    err = err_;
                    break;
                }
            }
        }
        Err(ResolverError::from_os_err(err))
    }
}

/// The UDP endpoint type.
pub type UdpEndpoint = IpEndpoint<Udp>;

/// The UDP socket type.
pub type UdpSocket = DgramSocket<Udp>;

/// The UDP resolver type.
pub type UdpResolver = Resolver<Udp>;

#[test]
fn test_ipv4() {
    use std::net::Ipv4Addr;

    let ep = UdpEndpoint::v4(Ipv4Addr::new(127, 0, 0, 1), 514);
    assert_eq!(ep.is_v4(), true);
    assert_eq!(ep.is_v6(), false);
    assert_eq!(ep.port(), 514);
    assert_eq!(ep.addr(), Ipv4Addr::new(127, 0, 0, 1));
}

#[test]
fn test_ipv6() {
    use std::net::Ipv6Addr;

    let ep = UdpEndpoint::v6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 514, 0);
    assert_eq!(ep.is_v4(), false);
    assert_eq!(ep.is_v6(), true);
    assert_eq!(ep.port(), 514);
    assert_eq!(ep.addr(), Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
}

/// The Internet Control Message Protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Icmp(AddressFamily, IpProtocol);

impl Icmp {
    /// Represents a ICMP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Icmp, IcmpEndpoint};
    /// use std::net::Ipv4Addr;
    ///
    ///
    /// let ep = IcmpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 0);
    /// assert_eq!(Icmp::V4, ep.protocol());
    /// ```
    pub const V4: Self = Self(AddressFamily::INET, IpProtocol::ICMP);

    /// Represents a ICMPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Icmp, IcmpEndpoint};
    /// use std::net::Ipv6Addr;
    ///
    ///
    /// let ep = IcmpEndpoint::v6(Ipv6Addr::UNSPECIFIED, 0, 0);
    /// assert_eq!(Icmp::V6, ep.protocol());
    /// ```
    pub const V6: Self = Self(AddressFamily::INET6, IpProtocol::ICMPV6);
}

impl Protocol for Icmp {
    type Type = IpProtocol;
    type Endpoint = IpEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::RAW
    }

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}

impl IpEndpoint<Icmp> {
    pub const fn protocol(&self) -> Icmp {
        match self.family_type() {
            AddressFamily::INET => Icmp::V4,
            AddressFamily::INET6 => Icmp::V6,
            _ => unreachable!(),
        }
    }
}

impl Resolver<Icmp> {
    /// The performs name resolution for ICMP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{IcmpResolver, IcmpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// for ep in IcmpResolver::v4(ctx).resolve(("localhost", "")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, &IcmpEndpoint::v4(Ipv4Addr::LOCALHOST, 0));
    ///     }
    ///     if !ep.is_v4() {
    ///         panic!("{:?}", ep);
    ///     }
    /// }
    /// ```
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Icmp::V4)
    }

    /// The performs name resolution for ICMPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{IcmpResolver, IcmpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv6Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// if let Ok(it) = IcmpResolver::v6(ctx).resolve(("localhost", "")) {
    ///     for ep in it {
    ///         if !ep.is_v6() {
    ///            panic!("{:?}", ep);
    ///         }
    ///         if !ep.is_v4() {
    ///             assert_eq!(ep, &IcmpEndpoint::v6(Ipv6Addr::LOCALHOST, 0, 0));
    ///         }
    ///     }
    /// }
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Icmp::V6)
    }

    pub fn connect<I>(&self, it: I) -> Result<(DgramSocket<Icmp>, I::Item), OsError>
    where
        I: Iterator<Item = IcmpEndpoint>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in it {
            let soc = DgramSocket::new(self.as_ctx(), ep.protocol())?;
            match soc.connect(&ep) {
                Ok(_) => return Ok((soc, ep)),
                Err(err_) => err = err_,
            }
        }
        Err(err)
    }
}

/// The ICMP(v6) endpoint type.
pub type IcmpEndpoint = IpEndpoint<Icmp>;

/// The ICMP(v6) socket type.
pub type IcmpSocket = DgramSocket<Icmp>;

/// The ICMP(v6) resolver type.
pub type IcmpResolver = Resolver<Icmp>;
