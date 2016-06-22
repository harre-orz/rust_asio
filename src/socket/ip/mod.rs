use std::io;
use std::fmt;
use std::mem;
use std::ptr;
use std::cmp;
use std::iter::Iterator;
use std::marker::PhantomData;
use {IoObject, Strand, Cancel};
use socket::*;
use socket::socket_base::*;
use ops::*;

/// Implements Link-Layer addresses.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LlAddr {
    addr: [u8; 6],
}

impl LlAddr {
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> LlAddr {
        Self::from_bytes(&[a,b,c,d,e,f])
    }

    fn from_bytes(addr: &[u8; 6]) -> LlAddr {
        LlAddr { addr: *addr }
    }
}

impl fmt::Display for LlAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:2x}:{:2x}:{:2x}:{:2x}:{:2x}:{:2x}",
               self.addr[0], self.addr[1], self.addr[2],
               self.addr[3], self.addr[4], self.addr[5])
    }
}

impl fmt::Debug for LlAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

fn is_netmask_impl(addr: &[u8]) -> bool {
    if addr[0] == 0 {
        return false;
    }

    let mut it = addr.iter();
    while let Some(n) = it.next() {
        match *n {
            0b00000000 |
            0b10000000 |
            0b11000000 |
            0b11100000 |
            0b11110000 |
            0b11111000 |
            0b11111100 |
            0b11111110 =>
                return it.all(|&x| x == 0),
            0b11111111 => {},
            _ => return false,
        }
    }
    true
}

fn netmask_len_impl(addr: &[u8]) -> Option<u8> {
    let mut len = 0;
    let mut it = addr.iter();
    while let Some(n) = it.next() {
        if *n == 0b11111111 {
            len += 8;
        } else {
            match *n {
                0b00000000 => len += 0,
                0b10000000 => len += 1,
                0b11000000 => len += 2,
                0b11100000 => len += 3,
                0b11110000 => len += 4,
                0b11111000 => len += 5,
                0b11111100 => len += 6,
                0b11111110 => len += 7,
                _ => return None,
            }
            return if it.all(|&x| x == 0) {
                Some(len)
            } else {
                None
            }
        }
    }
    Some(len)
}

/// Implements IP version 4 style addresses.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IpAddrV4 {
    addr: [u8; 4],
}

impl IpAddrV4 {
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> IpAddrV4 {
        IpAddrV4 { addr: [a,b,c,d] }
    }

    pub fn from_bytes(addr: &[u8; 4]) -> Self {
        IpAddrV4 { addr: *addr }
    }

    pub fn from_ulong(mut addr: u32) -> Self {
        let a = (addr & 0xFF) as u8;
        addr >>= 8;
        let b = (addr & 0xFF) as u8;
        addr >>= 8;
        let c = (addr & 0xFF) as u8;
        addr >>= 8;
        IpAddrV4::new(a,b,c,addr as u8)
    }

    pub fn any() -> Self {
        IpAddrV4 { addr: [0; 4] }
    }

    pub fn loopback() -> Self {
        IpAddrV4::new(127,0,0,1)
    }

    pub fn is_unspecified(&self) -> bool {
        self.addr.iter().all(|&x| x == 0)
    }

    pub fn is_loopback(&self) -> bool {
        (self.addr[0] & 0xFF) == 0x7F
    }

    pub fn is_class_a(&self) -> bool {
        (self.addr[0] & 0x80) == 0
    }

    pub fn is_class_b(&self) -> bool {
        (self.addr[0] & 0xC0) == 0x80
    }

    pub fn is_class_c(&self) -> bool {
        (self.addr[0] & 0xE0) == 0xC0
    }

    pub fn is_multicast(&self) -> bool {
        (self.addr[0] & 0xF0) == 0xE0
    }

    pub fn is_netmask(&self) -> bool {
        is_netmask_impl(&self.addr)
    }

    pub fn is_link_local(&self) -> bool {
        self.addr[0] == 0xA9 && self.addr[1] == 0xFE
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.addr.clone()
    }

    pub fn to_ulong(&self) -> u32 {
        (((self.addr[0] as u32
        ) << 8 + (self.addr[1] as u32)
        ) << 8 + (self.addr[2] as u32)
        ) << 8 + (self.addr[3] as u32)
    }

    pub fn broadcast(addr: &Self, mask: &Self) -> Self {
        Self::from_ulong(addr.to_ulong() | (mask.to_ulong() ^ 0xFFFFFFFF))
    }

    pub fn netmask(addr: &Self) -> Self {
        if addr.is_class_a() {
            Self::new(255,0,0,0)
        } else if addr.is_class_b() {
            Self::new(255,255,0,0)
        } else if addr.is_class_c() {
            Self::new(255,255,255,0)
        } else {
            Self::new(255,255,255,255)
        }
    }

    pub fn netmask_len(mask: &Self) -> Option<u8> {
        netmask_len_impl(&mask.addr)
    }
}

impl fmt::Display for IpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}",
               self.addr[0], self.addr[1], self.addr[2], self.addr[3])
    }
}

impl fmt::Debug for IpAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// Implements IP version 6 style addresses.
#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IpAddrV6 {
    scope_id: u32,
    addr: [u8; 16],
}

impl IpAddrV6 {
    pub fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16, scope_id: u32) -> Self {
        let ar = [ a.to_be(), b.to_be(), c.to_be(), d.to_be(), e.to_be(), f.to_be(), g.to_be(), h.to_be() ];
        Self::from_bytes(unsafe { mem::transmute(&ar) }, scope_id)
    }

    pub fn any() -> Self {
        IpAddrV6 { scope_id: 0, addr: [0; 16] }
    }

    pub fn loopback() -> Self {
        IpAddrV6 { scope_id: 0, addr: [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1] }
    }

    pub fn scope_id(&self) -> u32 {
        self.scope_id
    }

    pub fn from_bytes(addr: &[u8; 16], scope_id: u32) -> Self {
        IpAddrV6 { scope_id: scope_id, addr: *addr }
    }

    pub fn is_unspecified(&self) -> bool {
        self.addr.iter().all(|&x| x == 0)
    }

    pub fn is_loopback(&self) -> bool {
        (self.addr[0] == 0 && self.addr[1] == 0 && self.addr[2] == 0 && self.addr[3] == 0 &&
         self.addr[4] == 0 && self.addr[5] == 0 && self.addr[6] == 0 && self.addr[7] == 0 &&
         self.addr[8] == 0 && self.addr[9] == 0 && self.addr[10] == 0 && self.addr[11] == 0 &&
         self.addr[12] == 0 && self.addr[13] == 0 && self.addr[14] == 0 && self.addr[15] == 1)
    }

    pub fn is_link_local(&self) -> bool {
        self.addr[0] == 0xFE && (self.addr[1] & 0xC0) == 0x80
    }

    pub fn is_site_local(&self) -> bool {
        self.addr[0] == 0xFE && (self.addr[1] & 0xC0) == 0xC0
    }

    pub fn is_v4_mapped(&self) -> bool {
        (self.addr[0] == 0 && self.addr[1] == 0 && self.addr[2] == 0 && self.addr[3] == 0 &&
         self.addr[4] == 0 && self.addr[5] == 0 && self.addr[6] == 0 && self.addr[7] == 0 &&
         self.addr[8] == 0 && self.addr[9] == 0 && self.addr[10] == 0xFF && self.addr[11] == 0xFF)
    }

    pub fn is_v4_compatible(&self) -> bool {
        ((self.addr[0] == 0 && self.addr[1] == 0 && self.addr[2] == 0 && self.addr[3] == 0 &&
          self.addr[4] == 0 && self.addr[5] == 0 && self.addr[6] == 0 && self.addr[7] == 0 &&
          self.addr[8] == 0 && self.addr[9] == 0 && self.addr[10] == 0 && self.addr[11] == 0)
         && !(self.addr[12] == 0 && self.addr[13] == 0 && self.addr[14] == 0
              && (self.addr[15] == 0 || self.addr[15] == 1)))
    }

    pub fn is_multicast(&self) -> bool {
        self.addr[0] == 0xFF
    }

    pub fn is_multicast_global(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x0E
    }

    pub fn is_multicast_link_local(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x02
    }

    pub fn is_multicast_node_local(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x01
    }

    pub fn is_multicast_org_local(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x08
    }

    pub fn is_multicast_site_local(&self) -> bool {
        self.addr[0] == 0xFF && (self.addr[1] & 0x0F) == 0x05
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        self.addr.clone()
    }

    pub fn to_v4(&self) -> Option<IpAddrV4> {
        if self.is_v4_mapped() || self.is_v4_compatible() {
            Some(IpAddrV4 { addr: [ self.addr[12], self.addr[13], self.addr[14], self.addr[15] ] })
        } else {
            None
        }
    }

    pub fn v4_mapped(addr: &IpAddrV4) -> Self {
        IpAddrV6 {
            scope_id: 0,
            addr: [0,0,0,0,0,0,0,0,0,0,0xFF,0xFF,
                   addr.addr[0],addr.addr[1],addr.addr[2],addr.addr[3]]
        }
    }

    pub fn v4_compatible(addr: &IpAddrV4) -> Option<Self> {
        if addr.addr[0] == 0 && addr.addr[1] == 0 && addr.addr[2] == 0
            && (addr.addr[3] == 0 || addr.addr[3] == 1)
        {
            None
        } else {
            Some(IpAddrV6 {
                scope_id: 0,
                addr: [0,0,0,0,0,0,0,0,0,0,0,0,
                       addr.addr[0],addr.addr[1],addr.addr[2],addr.addr[3]]
            })
        }
    }
}

impl fmt::Display for IpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ar: &[u16; 8] = unsafe { mem::transmute(&self.addr) };
        write!(f, "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
               u16::from_be(ar[0]), u16::from_be(ar[1]), u16::from_be(ar[2]), u16::from_be(ar[3]),
               u16::from_be(ar[4]), u16::from_be(ar[5]), u16::from_be(ar[6]), u16::from_be(ar[7]),)
    }
}

impl fmt::Debug for IpAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// Implements version-independent IP addresses.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum IpAddr {
    V4(IpAddrV4),
    V6(IpAddrV6),
}

impl IpAddr {
    pub fn is_unspecified(&self) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr.is_unspecified(),
            &IpAddr::V6(ref addr) => addr.is_unspecified(),
        }
    }

    pub fn is_loopback(&self) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr.is_loopback(),
            &IpAddr::V6(ref addr) => addr.is_loopback(),
        }
    }

    pub fn is_multicast(&self) -> bool {
        match self {
            &IpAddr::V4(ref addr) => addr.is_multicast(),
            &IpAddr::V6(ref addr) => addr.is_multicast(),
        }
    }
}

impl fmt::Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &IpAddr::V4(ref addr) => write!(f, "{}", addr),
            &IpAddr::V6(ref addr) => write!(f, "{}", addr),
        }
    }
}

impl fmt::Debug for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// Provides convert to endpoint.
pub trait ToEndpoint<P: Protocol> {
    fn to_endpoint(self) -> IpEndpoint<P>;
}

impl<P: Protocol> ToEndpoint<P> for (IpAddrV4, u16) {
    fn to_endpoint(self) -> IpEndpoint<P> {
        IpEndpoint::from_v4(&self.0, self.1)
    }
}

impl<P: Protocol> ToEndpoint<P> for (IpAddrV6, u16) {
    fn to_endpoint(self) -> IpEndpoint<P> {
        IpEndpoint::from_v6(&self.0, self.1)
    }
}

impl<P: Protocol> ToEndpoint<P> for (IpAddr, u16) {
    fn to_endpoint(self) -> IpEndpoint<P> {
        match self.0 {
            IpAddr::V4(addr) => IpEndpoint::from_v4(&addr, self.1),
            IpAddr::V6(addr) => IpEndpoint::from_v6(&addr, self.1),
        }
    }
}

impl<'a, P: Protocol> ToEndpoint<P> for (&'a IpAddrV4, u16) {
    fn to_endpoint(self) -> IpEndpoint<P> {
        IpEndpoint::from_v4(self.0, self.1)
    }
}

impl<'a, P: Protocol> ToEndpoint<P> for (&'a IpAddrV6, u16) {
    fn to_endpoint(self) -> IpEndpoint<P> {
        IpEndpoint::from_v6(self.0, self.1)
    }
}

impl<'a, P: Protocol> ToEndpoint<P> for (&'a IpAddr, u16) {
    fn to_endpoint(self) -> IpEndpoint<P> {
        match self.0 {
            &IpAddr::V4(ref addr) => IpEndpoint::from_v4(addr, self.1),
            &IpAddr::V6(ref addr) => IpEndpoint::from_v6(addr, self.1),
        }
    }
}

/// Describes an endpoint for a version-independent IP socket.
#[derive(Clone)]
pub struct IpEndpoint<P: Protocol> {
    ss: sockaddr_storage,
    maker: PhantomData<P>,
}

impl<P: Protocol> IpEndpoint<P> {
    pub fn new<T: ToEndpoint<P>>(t: T) -> Self {
        t.to_endpoint()
    }

    pub fn is_v4(&self) -> bool {
        self.ss.ss_family == AF_INET as u16
    }

    pub fn is_v6(&self) -> bool {
        self.ss.ss_family == AF_INET6 as u16
    }

    pub fn addr(&self) -> IpAddr {
        match self.ss.ss_family as i32 {
            AF_INET => {
                let sin: &sockaddr_in = unsafe { mem::transmute(&self.ss) };
                IpAddr::V4(IpAddrV4::from_bytes(unsafe { mem::transmute(&sin.sin_addr) }))
            },
            AF_INET6  => {
                let sin6: &sockaddr_in6 = unsafe { mem::transmute(&self.ss) };
                IpAddr::V6(IpAddrV6::from_bytes(unsafe { mem::transmute(&sin6.sin6_addr) }, sin6.sin6_scope_id))
            },
            _ => panic!("Invalid family code ({}).", self.ss.ss_family),
        }
    }

    pub fn port(&self) -> u16 {
        let sin: &sockaddr_in = unsafe { mem::transmute(&self.ss) };
        u16::from_be(sin.sin_port)
    }

    fn default() -> IpEndpoint<P> {
        IpEndpoint {
            ss: unsafe { mem::zeroed() },
            maker: PhantomData,
        }
    }

    fn from_v4(addr: &IpAddrV4, port: u16) -> Self {
        let mut ep = IpEndpoint::default();
        let sin: &mut sockaddr_in = unsafe { mem::transmute(&mut ep.ss) };
        sin.sin_family = AF_INET as u16;
        sin.sin_port = port.to_be();
        unsafe {
            let src: *const u32 = mem::transmute(addr.addr.as_ptr());
            let dst: *mut u32 = mem::transmute(&mut sin.sin_addr);
            ptr::copy(src, dst, 1);
        }
        ep
    }

    fn from_v6(addr: &IpAddrV6, port: u16) -> Self {
        let mut ep = IpEndpoint::default();
        let sin6: &mut sockaddr_in6 = unsafe { mem::transmute(&mut ep.ss) };
        sin6.sin6_family = AF_INET6 as u16;
        sin6.sin6_port = port.to_be();
        sin6.sin6_scope_id = addr.scope_id();
        unsafe {
            let src: *const u64 = mem::transmute(addr.addr.as_ptr());
            let dst: *mut u64 = mem::transmute(&mut sin6.sin6_addr);
            ptr::copy(src, dst, 2);
        }
        ep
    }
}

impl<P: Protocol> AsRawSockAddr for IpEndpoint<P> {
    fn raw_socklen(&self) -> RawSockLenType {
        mem::size_of::<sockaddr_storage>() as RawSockLenType
    }

    fn as_raw_sockaddr(&self) -> &RawSockAddrType {
        unsafe { mem::transmute(&self.ss) }
    }

    fn as_mut_raw_sockaddr(&mut self) -> &mut RawSockAddrType {
        unsafe { mem::transmute(&mut self.ss) }
    }
}

impl<P: Protocol> Eq for IpEndpoint<P> {
}

impl<P: Protocol> PartialEq for IpEndpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        raw_sockaddr_eq(self, other)
    }
}

impl<P: Protocol> Ord for IpEndpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        raw_sockaddr_cmp(self, other)
    }
}

impl<P: Protocol> PartialOrd for IpEndpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Protocol> fmt::Display for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "{}:{}", addr, self.port()),
            IpAddr::V6(addr) => write!(f, "[{}]:{}", addr, self.port()),
        }
    }
}

impl<P: Protocol> fmt::Debug for IpEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// The IP-based socket tag.
pub trait IpSocket : Socket{
}

/// Socket option for determining whether an IPv6 socket supports IPv6 communication only.
#[derive(Default, Clone)]
pub struct V6Only(i32);

impl BooleanOption for V6Only {
    fn on() -> Self {
        V6Only(1)
    }

    fn is_on(&self) -> bool {
        self.0 != 0
    }
}

impl<S: IpSocket> GetSocketOption<S> for V6Only {
    type Data = i32;

    fn level(&self) -> i32 {
        IPPROTO_IPV6
    }

    fn name(&self) -> i32 {
        IPV6_V6ONLY
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<S: IpSocket> SetSocketOption<S> for V6Only {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}

/// Provides endpoint resolution functionality.
pub trait Resolver : Sized + Cancel {
    type Protocol: Protocol;

    fn resolve<'a, T: IoObject, Q: ResolveQuery<'a, Self>>(&self, io: &T, query: Q) -> io::Result<Q::Iter>;

    fn async_resolve<'a, Q, A, F, T>(a: A, query: Q, callback: F, obj: &Strand<T>)
        where Q: ResolveQuery<'a, Self>,
              A: Fn(&T) -> &Self + Send,
              F: FnOnce(Strand<T>, io::Result<Q::Iter>) + Send;
}

/// A query to be passed to a resolver.
pub trait ResolveQuery<'a, R: Resolver> {
    type Iter: Iterator;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter>;
}

mod resolve;
pub use self::resolve::*;

mod tcp;
pub use self::tcp::*;

mod udp;
pub use self::udp::*;

mod icmp;
pub use self::icmp::*;

#[test]
fn test_lladdr() {
    assert!(LlAddr::default().addr == [0,0,0,0,0,0]);
    assert!(LlAddr::new(1,2,3,4,5,6).addr == [1,2,3,4,5,6]);
    assert!(LlAddr::new(1,2,3,4,5,6) == LlAddr::from_bytes(&[1,2,3,4,5,6]));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,3,4,5,7));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,3,4,6,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,3,5,0,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,2,4,0,0,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(1,3,0,0,0,0));
    assert!(LlAddr::new(1,2,3,4,5,6) < LlAddr::new(2,0,0,0,0,0));
}

#[test]
fn test_ipaddr_v4() {
    assert!(IpAddrV4::default().addr == [0,0,0,0]);
    assert!(IpAddrV4::new(1,2,3,4).addr == [1,2,3,4]);
    assert!(IpAddrV4::new(1,2,3,4) == IpAddrV4::from_bytes(&[1,2,3,4]));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,2,3,5));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,2,4,0));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,3,0,0));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(2,0,0,0));
}

#[test]
fn test_ipaddr_v6() {
    assert!(IpAddrV6::default().addr == [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);
    assert!(IpAddrV6::new(0x0102,0x0304,0x0506,0x0708,0x090a,0x0b0c,0x0d0e,0x0f10,0).addr
            == [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
    assert!(IpAddrV6::new(0x0102,0x0304,0x0506,0x0708,0x090a,0x0b0c,0x0d0e,0x0f10,0)
            == IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0));
    assert!(IpAddrV6::new(0,0,0,0,0,0,0,0,100).scope_id() == 100);
    assert!(IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0) <
            IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,17], 0));
    assert!(IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0) <
            IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,16,00], 0));
    assert!(IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], 0) <
            IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,15,00,00], 0));
}

#[test]
fn test_endpoint_v4() {
    let ep = UdpEndpoint::new((IpAddrV4::new(1,2,3,4), 10));
    assert!(ep.is_v4());
    assert!(ep.addr() == IpAddr::V4(IpAddrV4::new(1,2,3,4)));
    assert!(ep.port() == 10);
    assert!(!ep.is_v6());
}

#[test]
fn test_endpoint_v6() {
    let ep = TcpEndpoint::new((IpAddrV6::new(1,2,3,4,5,6,7,8,0), 10));
    assert!(ep.is_v6());
    assert!(ep.addr() == IpAddr::V6(IpAddrV6::new(1,2,3,4,5,6,7,8,0)));
    assert!(ep.port() == 10);
    assert!(!ep.is_v4());
}

#[test]
fn test_endpoint_cmp() {
    let a = IcmpEndpoint::new((IpAddrV6::new(1,2,3,4,5,6,7,8,0), 10));
    let b = IcmpEndpoint::new((IpAddrV6::new(1,2,3,4,5,6,7,8,1), 10));
    let c = IcmpEndpoint::new((IpAddrV6::new(1,2,3,4,5,6,7,8,0), 11));
    assert!(a == a && b == b && c == c);
    assert!(a != b && b != c);
    assert!(a < b);
    assert!(b < c);
}
