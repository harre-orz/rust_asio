use crate::socket_base::{
    AddressFamily, Endpoint, IntoProtocolType, Protocol, SockaddrType, SocklenType,
};
use std::cmp;
use std::fmt;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::slice;

const SIZE_OF_SOCKADDR_IN: SocklenType = 16;
const SIZE_OF_SOCKADDR_IN6: SocklenType = 28;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct IpProtocol(u16);

impl IpProtocol {
    pub const TCP: Self = Self(libc::IPPROTO_TCP as u16);
    pub const UDP: Self = Self(libc::IPPROTO_UDP as u16);
    pub const RAW: Self = Self(libc::IPPROTO_RAW as u16);
    pub const ICMP: Self = Self(libc::IPPROTO_ICMP as u16);
    pub const ICMPV6: Self = Self(libc::IPPROTO_ICMPV6 as u16);
}

impl Into<i32> for IpProtocol {
    fn into(self) -> i32 {
        self.0 as i32
    }
}

impl IntoProtocolType for IpProtocol {}

#[derive(Copy, Clone)]
union Inner {
    sa: libc::sockaddr,
    sin: libc::sockaddr_in,
    sin6: libc::sockaddr_in6,
}

#[derive(Copy, Clone)]
pub struct IpEndpoint<P> {
    inner: Inner,
    len: SocklenType,
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
            inner: Inner {
                sin: libc::sockaddr_in {
                    sin_family: AddressFamily::INET.0,
                    sin_port: port.to_be(),
                    sin_addr: unsafe { mem::transmute(addr) },
                    sin_zero: [0; 8],
                },
            },
            len: SIZE_OF_SOCKADDR_IN,
            _marker: PhantomData,
        }
    }

    pub const fn v6(addr: Ipv6Addr, port: u16, scope_id: u32) -> Self {
        IpEndpoint {
            inner: Inner {
                sin6: libc::sockaddr_in6 {
                    sin6_family: AddressFamily::INET6.0,
                    sin6_port: port.to_be(),
                    sin6_addr: unsafe { mem::transmute(addr) },
                    sin6_flowinfo: 0,
                    sin6_scope_id: scope_id,
                },
            },
            len: SIZE_OF_SOCKADDR_IN6,
            _marker: PhantomData,
        }
    }

    pub(crate) const fn family_type(&self) -> AddressFamily {
        AddressFamily(unsafe { self.inner.sa.sa_family })
    }

    pub const fn is_v4(&self) -> bool {
        self.family_type().0 == AddressFamily::INET.0
    }

    pub const fn is_v6(&self) -> bool {
        self.family_type().0 == AddressFamily::INET6.0
    }

    pub fn addr(&self) -> IpAddr {
        match self.family_type() {
            AddressFamily::INET => IpAddr::V4(unsafe { self.as_ipv4_addr() }.clone()),
            AddressFamily::INET6 => IpAddr::V6(unsafe { self.as_ipv6_addr() }.clone()),
            _ => unreachable!(),
        }
    }

    pub const unsafe fn as_ipv4_addr(&self) -> &Ipv4Addr {
        mem::transmute(&self.inner.sin.sin_addr)
    }

    pub const unsafe fn as_ipv6_addr(&self) -> &Ipv6Addr {
        mem::transmute(&self.inner.sin6.sin6_addr)
    }

    pub const fn port(&self) -> u16 {
        u16::from_be(unsafe { self.inner.sin.sin_port })
    }

    fn as_bytes(&self) -> &[u8] {
        unsafe {
            let sa = &self.inner.sa as SockaddrType as *const u8;
            slice::from_raw_parts(sa, self.len as usize)
        }
    }
}

impl<P> Endpoint for IpEndpoint<P>
where
    P: Protocol,
{
    const SIZE: SocklenType = SIZE_OF_SOCKADDR_IN6;

    fn len(&self) -> SocklenType {
        self.len
    }

    fn as_ptr(&self) -> SockaddrType {
        unsafe { &self.inner.sa }
    }

    unsafe fn init(sa: MaybeUninit<Self>, len: SocklenType) -> Self {
        if len as usize >= mem::size_of::<Inner>() {
            panic!()
        }

        let mut ep = sa.assume_init();
        ep.len = len;
        match ep.family_type() {
            AddressFamily::INET if ep.len == SIZE_OF_SOCKADDR_IN => ep,
            AddressFamily::INET6 if ep.len == SIZE_OF_SOCKADDR_IN6 => ep,
            _ => panic!(),
        }
    }
}

impl<P> AsRef<Self> for IpEndpoint<P> {
    fn as_ref(&self) -> &Self {
        self
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

impl<P> fmt::Debug for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.addr() {
            IpAddr::V4(addr) => write!(
                f,
                "IpEndpoint {{ addr = {}, port = {} }}",
                addr,
                self.port()
            ),
            IpAddr::V6(addr) => write!(
                f,
                "IpEndpoint {{ addr = {}, port = {} }}",
                addr,
                self.port()
            ),
        }
    }
}

impl<P> cmp::PartialEq<Self> for IpEndpoint<P> {
    fn eq(&self, rhs: &Self) -> bool {
        self.as_bytes() == rhs.as_bytes()
    }
}

impl<P> cmp::Eq for IpEndpoint<P> {}
