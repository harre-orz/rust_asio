use super::{
    NativeHandleType, NativeSockAddrType, NativeSockLenType,
    ReadWrite, Buffer, MutableBuffer,
    Shutdown, Protocol, AsBytes, AsSockAddr, Endpoint as BasicEndpoint,
    IoControl, GetSocketOption, SetSocketOption,
    IoService, IoObject, SocketBase, Socket, StreamSocket, ListenerSocket,
};
use super::BasicSocket;
use std::io;
use std::fmt;
use std::mem;
use std::ptr;
use std::cmp;
use std::marker::PhantomData;
use std::iter::Iterator;
use libc;

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

impl AsBytes for LlAddr {
    type Bytes = [u8; 6];

    fn as_bytes(&self) -> &[u8; 6] {
        &self.addr
    }

    fn as_mut_bytes(&mut self) -> &mut [u8; 6] {
        &mut self.addr
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

#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IpAddrV4 {
    addr: [u8; 4],
}

impl IpAddrV4 {
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> IpAddrV4 {
        IpAddrV4 { addr: [a,b,c,d] }
    }

    fn from_bytes(addr: &[u8; 4]) -> IpAddrV4 {
        IpAddrV4 { addr: *addr }
    }
}

impl AsBytes for IpAddrV4 {
    type Bytes = [u8; 4];

    fn as_bytes(&self) -> &[u8; 4] {
        &self.addr
    }

    fn as_mut_bytes(&mut self) -> &mut [u8; 4] {
        &mut self.addr
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

#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IpAddrV6 {
    scope_id: u32,
    addr: [u8; 16],
}

impl IpAddrV6 {
    pub fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16, scope_id: u32) -> IpAddrV6 {
        let ar = [ a.to_be(), b.to_be(), c.to_be(), d.to_be(), e.to_be(), f.to_be(), g.to_be(), h.to_be() ];
        Self::from_bytes(unsafe { mem::transmute(&ar) }, scope_id)
    }

    pub fn scope_id(&self) -> u32 {
        self.scope_id
    }

    fn from_bytes(addr: &[u8; 16], scope_id: u32) -> IpAddrV6 {
        IpAddrV6 { scope_id: scope_id, addr: *addr }
    }
}

impl AsBytes for IpAddrV6 {
    type Bytes = [u8; 16];

    fn as_bytes(&self) -> &[u8; 16] {
        &self.addr
    }

    fn as_mut_bytes(&mut self) -> &mut [u8; 16] {
        &mut self.addr
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum IpAddr {
    V4(IpAddrV4),
    V6(IpAddrV6),
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

pub trait ToEndpoint<P: Protocol> {
    fn to_endpoint(self) -> Endpoint<P>;
}

impl<P: Protocol> ToEndpoint<P> for (IpAddrV4, u16) {
    fn to_endpoint(self) -> Endpoint<P> {
        Endpoint::from_v4(&self.0, self.1)
    }
}

impl<'a, P: Protocol> ToEndpoint<P> for (&'a IpAddrV4, u16) {
    fn to_endpoint(self) -> Endpoint<P> {
        Endpoint::from_v4(self.0, self.1)
    }
}

impl<P: Protocol> ToEndpoint<P> for (IpAddrV6, u16) {
    fn to_endpoint(self) -> Endpoint<P> {
        Endpoint::from_v6(&self.0, self.1)
    }
}

impl<'a, P: Protocol> ToEndpoint<P> for (&'a IpAddrV6, u16) {
    fn to_endpoint(self) -> Endpoint<P> {
        Endpoint::from_v6(self.0, self.1)
    }
}

impl<P: Protocol> ToEndpoint<P> for (IpAddr, u16) {
    fn to_endpoint(self) -> Endpoint<P> {
        match self.0 {
            IpAddr::V4(addr) => Endpoint::from_v4(&addr, self.1),
            IpAddr::V6(addr) => Endpoint::from_v6(&addr, self.1),
        }
    }
}

impl<'a, P: Protocol> ToEndpoint<P> for (&'a IpAddr, u16) {
    fn to_endpoint(self) -> Endpoint<P> {
        match self.0 {
            &IpAddr::V4(ref addr) => Endpoint::from_v4(addr, self.1),
            &IpAddr::V6(ref addr) => Endpoint::from_v6(addr, self.1),
        }
    }
}

#[derive(Clone)]
pub struct Endpoint<P: Protocol> {
    ss: libc::sockaddr_storage,
    maker: PhantomData<P>,
}

impl<P: Protocol> Endpoint<P> {
    pub fn new<T: ToEndpoint<P>>(t: T) -> Self {
        t.to_endpoint()
    }

    pub fn is_v4(&self) -> bool {
        self.ss.ss_family == libc::AF_INET as u16
    }

    pub fn is_v6(&self) -> bool {
        self.ss.ss_family == libc::AF_INET6 as u16
    }

    pub fn addr(&self) -> IpAddr {
        match self.ss.ss_family as i32 {
            libc::AF_INET => {
                let sin: &libc::sockaddr_in = unsafe { mem::transmute(&self.ss) };
                IpAddr::V4(IpAddrV4::from_bytes(unsafe { mem::transmute(&sin.sin_addr) }))
            },
            libc::AF_INET6  => {
                let sin6: &libc::sockaddr_in6 = unsafe { mem::transmute(&self.ss) };
                IpAddr::V6(IpAddrV6::from_bytes(unsafe { mem::transmute(&sin6.sin6_addr) }, sin6.sin6_scope_id))
            },
            _ => panic!("Invalid family code ({}).", self.ss.ss_family),
        }
    }

    pub fn port(&self) -> u16 {
        let sin: &libc::sockaddr_in = unsafe { mem::transmute(&self.ss) };
        u16::from_be(sin.sin_port)
    }

    fn default() -> Endpoint<P> {
        Endpoint {
            ss: unsafe { mem::zeroed() },
            maker: PhantomData,
        }
    }

    fn from_v4(addr: &IpAddrV4, port: u16) -> Self {
        let mut ep = Endpoint::default();
        let sin: &mut libc::sockaddr_in = unsafe { mem::transmute(&mut ep.ss) };
        sin.sin_family = libc::AF_INET as u16;
        sin.sin_port = port.to_be();
        unsafe {
            let src: *const u32 = mem::transmute(addr.as_bytes());
            let dst: *mut u32 = mem::transmute(&mut sin.sin_addr);
            ptr::copy(src, dst, 1);
        }
        ep
    }

    fn from_v6(addr: &IpAddrV6, port: u16) -> Self {
        let mut ep = Endpoint::default();
        let sin6: &mut libc::sockaddr_in6 = unsafe { mem::transmute(&mut ep.ss) };
        sin6.sin6_family = libc::AF_INET6 as u16;
        sin6.sin6_port = port.to_be();
        sin6.sin6_scope_id = addr.scope_id();
        unsafe {
            let src: *const u64 = mem::transmute(addr.as_bytes());
            let dst: *mut u64 = mem::transmute(&mut sin6.sin6_addr);
            ptr::copy(src, dst, 2);
        }
        ep
    }
}

impl<P: Protocol> AsSockAddr for Endpoint<P> {
    fn socklen(&self) -> NativeSockLenType {
        mem::size_of_val(&self.ss) as NativeSockLenType
    }

    fn as_sockaddr(&self) -> &NativeSockAddrType {
        unsafe { mem::transmute(&self.ss) }
    }

    fn as_mut_sockaddr(&mut self) -> &mut NativeSockAddrType {
        unsafe { mem::transmute(&mut self.ss) }
    }
}

impl<P: Protocol> Eq for Endpoint<P> {
}

impl<P: Protocol> PartialEq for Endpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        self.eq_impl(other)
    }
}

impl<P: Protocol> Ord for Endpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.cmp_impl(other)
    }
}

impl<P: Protocol> PartialOrd for Endpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Protocol> fmt::Display for Endpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "{}:{}", addr, self.port()),
            IpAddr::V6(addr) => write!(f, "[{}]:{}", addr, self.port()),
        }
    }
}

impl<P: Protocol> fmt::Debug for Endpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Clone)]
pub struct ResolveEntry<'i, P: Protocol> {
    ai: &'i libc::addrinfo,
    marker: PhantomData<P>,
}

impl<'i, P: Protocol> ResolveEntry<'i, P> {
    pub fn endpoint(&self) -> Endpoint<P> {
        let mut ep = Endpoint::default();
        unsafe {
            let src: *const u8 = mem::transmute(self.ai.ai_addr);
            let dst: *mut u8 = mem::transmute(ep.as_mut_sockaddr());
            ptr::copy(src, dst, self.ai.ai_addrlen as usize);
        }
        ep
    }

    pub fn flags(&self) -> i32 {
        self.ai.ai_flags
    }

    pub fn is_v4(&self) -> bool {
        self.ai.ai_family == libc::AF_INET
    }

    pub fn is_v6(&self) -> bool {
        self.ai.ai_family == libc::AF_INET6
    }
}

pub struct ResolveIter<'i, P: Protocol> {
    base: &'i mut libc::addrinfo,
    ai: *mut libc::addrinfo,
    marker: PhantomData<P>,
}

const ADDRINFO_NODE_MAX: usize = 256;
const ADDRINFO_SERV_MAX: usize = 256;
impl<'i, P: Protocol> ResolveIter<'i, P> {
    fn active(pro: P, host: &str, port: &str, flags: i32) -> io::Result<Self> {
        let mut hints: libc::addrinfo = unsafe { mem::zeroed() };
        hints.ai_flags = flags;
        hints.ai_family = pro.family_type();
        hints.ai_socktype = pro.socket_type();
        hints.ai_protocol = pro.protocol_type();

        let mut node: [libc::c_char; ADDRINFO_NODE_MAX] = [0; ADDRINFO_NODE_MAX];
        for (a, c) in node[0..ADDRINFO_NODE_MAX-1].iter_mut().zip(host.chars()) { *a = c as i8; }

        let mut serv: [libc::c_char; ADDRINFO_SERV_MAX] = [0; ADDRINFO_SERV_MAX];
        for (a, c) in serv[0..ADDRINFO_SERV_MAX-1].iter_mut().zip(port.chars()) { *a = c as i8; }

        let mut base: *mut libc::addrinfo = unsafe { mem::uninitialized() };
        libc_try!(libc::getaddrinfo(mem::transmute(&node), mem::transmute(&serv), &hints, &mut base));
        Ok(ResolveIter { base: unsafe { &mut *base }, ai: base, marker: PhantomData, })
    }

    fn passive(pro: P, port: &str, flags: i32) -> io::Result<Self> {
        let mut hints: libc::addrinfo = unsafe { mem::zeroed() };
        hints.ai_flags = flags;
        hints.ai_family = pro.family_type();
        hints.ai_socktype = pro.socket_type();
        hints.ai_protocol = pro.protocol_type();

        let mut serv: [libc::c_char; ADDRINFO_SERV_MAX] = [0; ADDRINFO_SERV_MAX];
        for (a, c) in serv[0..ADDRINFO_SERV_MAX-1].iter_mut().zip(port.chars()) { *a = c as i8; }

        let mut base: *mut libc::addrinfo = unsafe { mem::uninitialized() };
        libc_try!(libc::getaddrinfo(ptr::null(), mem::transmute(&serv), &hints, &mut base));
        Ok(ResolveIter { base: unsafe { &mut *base }, ai: base, marker: PhantomData, })
    }
}

impl<'i, P: Protocol> Drop for ResolveIter<'i, P> {
    fn drop(&mut self) {
        unsafe { libc::freeaddrinfo(self.base) }
    }
}

impl<'i, P: Protocol> Iterator for ResolveIter<'i, P> {
    type Item = ResolveEntry<'i, P>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.ai.is_null() {
            let ai = unsafe { &mut *self.ai };
            self.ai = ai.ai_next;
            return Some( ResolveEntry { ai: ai, marker: self.marker } );
        }
        None
    }
}

const AI_PASSIVE: i32 = 0x0001;
const AI_NUMERICHOST: i32 = 0x0004;
const AI_NUMERICSERV: i32 = 0x0400;

pub trait ResolveQuery<P: Protocol> {
    fn query<'i>(self, pro: P) -> io::Result<ResolveIter<'i, P>>;
}

impl<'a, 'b, P: Protocol> ResolveQuery<P> for (&'a str, &'b str) {
    fn query<'i>(self, pro: P) -> io::Result<ResolveIter<'i, P>> {
        ResolveIter::active(pro, self.0, self.1, 0)
    }
}

impl<'a, P: Protocol> ResolveQuery<P> for (&'a str, u16) {
    fn query<'i>(self, pro: P) -> io::Result<ResolveIter<'i, P>> {
        let port = self.1.to_string();
        ResolveIter::active(pro, self.0, &port[..], 0)
    }
}

impl<P: Protocol> ResolveQuery<P> for (IpAddrV4, u16) {
    fn query<'i>(self, pro: P) -> io::Result<ResolveIter<'i, P>> {
        let host = self.0.to_string();
        let port = self.1.to_string();
        ResolveIter::active(pro, &host[..], &port[..], AI_NUMERICHOST | AI_NUMERICSERV)
    }
}

impl<P: Protocol> ResolveQuery<P> for (IpAddrV6, u16) {
    fn query<'i>(self, pro: P) -> io::Result<ResolveIter<'i, P>> {
        let host = self.0.to_string();
        let port = self.1.to_string();
        ResolveIter::active(pro, &host[..], &port[..], AI_NUMERICHOST | AI_NUMERICSERV)
    }
}

impl<P: Protocol> ResolveQuery<P> for (IpAddr, u16) {
    fn query<'i>(self, pro: P) -> io::Result<ResolveIter<'i, P>> {
        let host = self.0.to_string();
        let port = self.1.to_string();
        ResolveIter::active(pro, &host[..], &port[..], AI_NUMERICHOST | AI_NUMERICSERV)
    }
}

impl<P: Protocol> ResolveQuery<P> for u16 {
    fn query<'i>(self, pro: P) -> io::Result<ResolveIter<'i, P>> {
        let port = self.to_string();
        ResolveIter::passive(pro, &port[..], AI_PASSIVE)
    }
}

pub trait Resolver<'a, P: Protocol> : IoObject<'a> {
    // type Query;
    // type Iter;
    fn resolve<'i, T: ResolveQuery<P>>(&self, t: T) -> io::Result<ResolveIter<'i, P>>;
}

#[allow(private_in_public)]
struct BasicResolver<'a, P: Protocol> {
    io: &'a IoService,
    marker: PhantomData<P>,
}

impl<'a, P: Protocol> BasicResolver<'a, P> {
    pub fn new(io: &'a IoService) -> io::Result<Self> {
        Ok(BasicResolver { io: io, marker: PhantomData })
    }
}

impl<'a, P: Protocol> IoObject<'a> for BasicResolver<'a, P> {
    fn io_service(&self) -> &'a IoService {
        self.io
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Tcp {
    family: i32,
}

impl Tcp {
    pub fn v4() -> Tcp {
        Tcp { family: libc::AF_INET as i32 }
    }

    pub fn v6() -> Tcp {
        Tcp { family: libc::AF_INET6 as i32 }
    }
}

impl Protocol for Tcp {
    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_STREAM as i32
    }

    fn protocol_type(&self) -> i32 {
        libc::IPPROTO_TCP as i32
    }
}

impl BasicEndpoint<Tcp> for Endpoint<Tcp> {
    fn protocol(&self) -> Tcp {
        if self.is_v4() {
            Tcp::v4()
        } else if self.is_v6() {
            Tcp::v6()
        } else {
            panic!("Invalid family code ({}).", self.ss.ss_family);
        }
    }
}

pub type TcpEndpoint = Endpoint<Tcp>;

impl<'a> Resolver<'a, Tcp> for BasicResolver<'a, Tcp> {
    fn resolve<'i, T: ResolveQuery<Tcp>>(&self, t: T) -> io::Result<ResolveIter<'i, Tcp>> {
        t.query(Tcp { family: 0 })
    }
}

pub type TcpResolver<'a> = BasicResolver<'a, Tcp>;

pub struct TcpStream<'a> {
    _impl: BasicSocket<'a, Tcp>,
}

impl<'a> IoObject<'a> for TcpStream<'a> {
    fn io_service(&self) -> &'a IoService {
        self._impl.io_service()
    }
}

impl<'a> SocketBase<Tcp> for TcpStream<'a> {
    type Endpoint = Endpoint<Tcp>;

    unsafe fn native_handle(&self) -> &NativeHandleType {
        self._impl.native_handle()
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        self._impl.local_endpoint(Endpoint::default())
    }

    fn io_control<T: IoControl>(&self, cmd: &mut T) -> io::Result<()> {
        self._impl.io_control(cmd)
    }

    fn get_socket<T: GetSocketOption>(&self) -> io::Result<T> {
        self._impl.get_socket()
    }

    fn set_socket<T: SetSocketOption>(&self, cmd: &T) -> io::Result<()> {
        self._impl.set_socket(cmd)
    }

    fn get_non_blocking(&self) -> io::Result<bool> {
        self._impl.get_non_blocking()
    }

    fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        self._impl.set_non_blocking(on)
    }
}

impl<'a> StreamSocket<'a, Tcp> for TcpStream<'a> {
    fn connect(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        Ok(TcpStream { _impl: try!(BasicSocket::connect(io, ep)) })
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self._impl.shutdown(how)
    }

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint> {
        self._impl.remote_endpoint(Endpoint::default())
    }

    fn available(&self) -> io::Result<usize> {
        self._impl.available()
    }

    fn recv<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<usize> {
        self._impl.recv(buf, flags)
    }

    fn send<B: Buffer>(&self, buf: B, flags: i32) -> io::Result<usize> {
        self._impl.send(buf, flags)
    }
}

impl<'a> ReadWrite<'a> for TcpStream<'a> {
    fn read_some<B: MutableBuffer>(&self, buf: B) -> io::Result<usize> {
        self._impl.recv(buf, 0)
    }

    fn write_some<B: Buffer>(&self, buf: B) -> io::Result<usize> {
        self._impl.send(buf, 0)
    }
}

pub struct TcpListener<'a> {
    _impl: BasicSocket<'a, Tcp>,
}

impl<'a> IoObject<'a> for TcpListener<'a> {
    fn io_service(&self) -> &'a IoService {
        self._impl.io_service()
    }
}

impl<'a> SocketBase<Tcp> for TcpListener<'a> {
    type Endpoint = Endpoint<Tcp>;

    unsafe fn native_handle(&self) -> &NativeHandleType {
        self._impl.native_handle()
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        self._impl.local_endpoint(Endpoint::default())
    }

    fn io_control<T: IoControl>(&self, cmd: &mut T) -> io::Result<()> {
        self._impl.io_control(cmd)
    }

    fn get_socket<T: GetSocketOption>(&self) -> io::Result<T> {
        self._impl.get_socket()
    }

    fn set_socket<T: SetSocketOption>(&self, cmd: &T) -> io::Result<()> {
        self._impl.set_socket(cmd)
    }

    fn get_non_blocking(&self) -> io::Result<bool> {
        self._impl.get_non_blocking()
    }

    fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        self._impl.set_non_blocking(on)
    }
}

impl<'a> ListenerSocket<'a, Tcp> for TcpListener<'a> {
    type Socket = TcpStream<'a>;

    fn listen(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        Ok(TcpListener { _impl: try!(BasicSocket::listen(io, ep)) })
    }

    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)> {
        let _impl = try!(self._impl.accept(Endpoint::default()));
        Ok((TcpStream { _impl: _impl.0 }, _impl.1))
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Udp {
    family: i32,
}

impl Udp {
    pub fn v4() -> Udp {
        Udp { family: libc::AF_INET as i32 }
    }

    pub fn v6() -> Udp {
        Udp { family: libc::AF_INET6 as i32 }
    }
}

impl Protocol for Udp {
    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_DGRAM as i32
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl BasicEndpoint<Udp> for Endpoint<Udp> {
    fn protocol(&self) -> Udp {
        if self.is_v4() {
            Udp::v4()
        } else if self.is_v6() {
            Udp::v6()
        } else {
            panic!("Invalid family code ({}).", self.ss.ss_family);
        }
    }
}

pub type UdpEndpoint = Endpoint<Udp>;

impl<'a> Resolver<'a, Udp> for BasicResolver<'a, Udp> {
    fn resolve<'i, T: ResolveQuery<Udp>>(&self, t: T) -> io::Result<ResolveIter<'i, Udp>> {
        t.query(Udp { family: 0 })
    }
}

pub type UdpResolver<'a> = BasicResolver<'a, Udp>;

pub struct UdpSocket<'a> {
    _impl: BasicSocket<'a, Udp>,
}

impl<'a> IoObject<'a> for UdpSocket<'a> {
    fn io_service(&self) -> &'a IoService {
        self._impl.io_service()
    }
}

impl<'a> SocketBase<Udp> for UdpSocket<'a> {
    type Endpoint = Endpoint<Udp>;

    unsafe fn native_handle(&self) -> &NativeHandleType {
        self._impl.native_handle()
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        self._impl.local_endpoint(Endpoint::default())
    }

    fn io_control<T: IoControl>(&self, cmd: &mut T) -> io::Result<()> {
        self._impl.io_control(cmd)
    }

    fn get_socket<T: GetSocketOption>(&self) -> io::Result<T> {
        self._impl.get_socket()
    }

    fn set_socket<T: SetSocketOption>(&self, cmd: &T) -> io::Result<()> {
        self._impl.set_socket(cmd)
    }

    fn get_non_blocking(&self) -> io::Result<bool> {
        self._impl.get_non_blocking()
    }

    fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        self._impl.set_non_blocking(on)
    }
}

impl<'a> Socket<'a, Udp> for UdpSocket<'a> {
    fn bind(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        Ok(UdpSocket { _impl: try!(BasicSocket::bind(io, ep)) })
    }

    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()> {
        self._impl.reconnect(ep)
    }

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint> {
        self._impl.remote_endpoint(Endpoint::default())
    }

    fn available(&self) -> io::Result<usize> {
        self._impl.available()
    }

    fn recv<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<usize> {
        self._impl.recv(buf, flags)
    }

    fn send<B: Buffer>(&self, buf: B, flags: i32) -> io::Result<usize> {
        self._impl.send(buf, flags)
    }

    fn recv_from<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<(usize, Self::Endpoint)> {
        self._impl.recv_from(buf, flags, Endpoint::default())
    }

    fn send_to<B: Buffer>(&self, buf: B, flags: i32, ep: &Self::Endpoint) -> io::Result<usize> {
        self._impl.send_to(buf, flags, ep)
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Icmp {
    family: i32,
    protocol: i32,
}

const IPPROTO_ICMP: i32 = 1;
const IPPROTO_ICMPV6: i32 = 58;
impl Icmp {
    pub fn v4() -> Icmp {
        Icmp { family: libc::AF_INET as i32, protocol: IPPROTO_ICMP }
    }

    pub fn v6() -> Icmp {
        Icmp { family: libc::AF_INET6 as i32, protocol: IPPROTO_ICMPV6 }
    }
}

impl Protocol for Icmp {
    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_RAW as i32
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }
}

impl BasicEndpoint<Icmp> for Endpoint<Icmp> {
    fn protocol(&self) -> Icmp {
        if self.is_v4() {
            Icmp::v4()
        } else if self.is_v6() {
            Icmp::v6()
        } else {
            panic!("Invalid family code ({}).", self.ss.ss_family);
        }
    }
}

pub type IcmpEndpoint = Endpoint<Icmp>;

impl<'a> Resolver<'a, Icmp> for BasicResolver<'a, Icmp> {
    fn resolve<'i, T: ResolveQuery<Icmp>>(&self, t: T) -> io::Result<ResolveIter<'i, Icmp>> {
        t.query(Icmp { family: 0, protocol: 0 })
    }
}

pub type IcmpResolver<'a> = BasicResolver<'a, Icmp>;

pub struct IcmpSocket<'a> {
    _impl: BasicSocket<'a, Icmp>,
}

impl<'a> IoObject<'a> for IcmpSocket<'a> {
    fn io_service(&self) -> &'a IoService {
        self._impl.io_service()
    }
}

impl<'a> SocketBase<Icmp> for IcmpSocket<'a> {
    type Endpoint = Endpoint<Icmp>;

    unsafe fn native_handle(&self) -> &NativeHandleType {
        self._impl.native_handle()
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        self._impl.local_endpoint(Endpoint::default())
    }

    fn io_control<T: IoControl>(&self, cmd: &mut T) -> io::Result<()> {
        self._impl.io_control(cmd)
    }

    fn get_socket<T: GetSocketOption>(&self) -> io::Result<T> {
        self._impl.get_socket()
    }

    fn set_socket<T: SetSocketOption>(&self, cmd: &T) -> io::Result<()> {
        self._impl.set_socket(cmd)
    }

    fn get_non_blocking(&self) -> io::Result<bool> {
        self._impl.get_non_blocking()
    }

    fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        self._impl.set_non_blocking(on)
    }
}

impl<'a> Socket<'a, Icmp> for IcmpSocket<'a> {
    fn bind(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        Ok(IcmpSocket { _impl: try!(BasicSocket::bind(io, ep)) })
    }

    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()> {
        self._impl.reconnect(ep)
    }

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint> {
        self._impl.remote_endpoint(Endpoint::default())
    }

    fn available(&self) -> io::Result<usize> {
        self._impl.available()
    }

    fn recv<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<usize> {
        self._impl.recv(buf, flags)
    }

    fn send<B: Buffer>(&self, buf: B, flags: i32) -> io::Result<usize> {
        self._impl.send(buf, flags)
    }

    fn recv_from<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<(usize, Self::Endpoint)> {
        self._impl.recv_from(buf, flags, Endpoint::default())
    }

    fn send_to<B: Buffer>(&self, buf: B, flags: i32, ep: &Self::Endpoint) -> io::Result<usize> {
        self._impl.send_to(buf, flags, ep)
    }
}

#[test]
fn test_lladdr() {
    assert!(LlAddr::default().as_bytes() == &[0,0,0,0,0,0]);
    assert!(LlAddr::new(1,2,3,4,5,6).as_bytes() == &[1,2,3,4,5,6]);
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
    assert!(IpAddrV4::default().as_bytes() == &[0,0,0,0]);
    assert!(IpAddrV4::new(1,2,3,4).as_bytes() == &[1,2,3,4]);
    assert!(IpAddrV4::new(1,2,3,4) == IpAddrV4::from_bytes(&[1,2,3,4]));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,2,3,5));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,2,4,0));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(1,3,0,0));
    assert!(IpAddrV4::new(1,2,3,4) < IpAddrV4::new(2,0,0,0));
}

#[test]
fn test_ipaddr_v6() {
    assert!(IpAddrV6::default().as_bytes() == &[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);
    assert!(IpAddrV6::new(0x0102,0x0304,0x0506,0x0708,0x090a,0x0b0c,0x0d0e,0x0f10,0).as_bytes()
            == &[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
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

#[test]
fn test_tcp() {
    assert!(Tcp::v4() == Tcp::v4());
    assert!(Tcp::v6() == Tcp::v6());
    assert!(Tcp::v4() != Tcp::v6());
}

#[test]
fn test_udp() {
    assert!(Udp::v4() == Udp::v4());
    assert!(Udp::v6() == Udp::v6());
    assert!(Udp::v4() != Udp::v6());
}

#[test]
fn test_icmp() {
    assert!(Icmp::v4() == Icmp::v4());
    assert!(Icmp::v6() == Icmp::v6());
    assert!(Icmp::v4() != Icmp::v6());
}

#[test]
fn test_tcp_resolve() {
    let io = IoService::default();
    let re = TcpResolver::new(&io).unwrap();
    for e in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(e.endpoint() == TcpEndpoint::new((IpAddrV4::new(127,0,0,1), 80)));
    }
}

#[test]
fn test_udp_resolve() {
    let io = IoService::default();
    let re = UdpResolver::new(&io).unwrap();
    for e in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(e.endpoint() == UdpEndpoint::new((IpAddrV4::new(127,0,0,1), 80)));
    }
}

#[test]
fn test_icmp_resolve() {
    let io = IoService::default();
    let re = IcmpResolver::new(&io).unwrap();
    for e in re.resolve(("127.0.0.1", "")).unwrap() {
        assert!(e.endpoint() == IcmpEndpoint::new((IpAddrV4::new(127,0,0,1), 0)));
    }
}
