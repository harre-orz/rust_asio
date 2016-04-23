use super::{IoService, Protocol, AsBytes, AsSockAddr, Endpoint as BasicEndpoint, Socket, StreamSocket, ListenerSocket, DgramSocket, RawSocket, Buffer, MutableBuffer};

use super::ops;
use super::cmd;
use std::io;
use std::fmt;
use std::mem;
use std::ptr;
use std::cmp;
use libc;

#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LlAddr {
    addr: [u8; 6],
}

impl LlAddr {
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> LlAddr {
        LlAddr {
            addr: [a, b, c, d, e, f],
        }
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
    fn from_bytes(addr: &[u8; 6]) -> LlAddr {
        LlAddr { addr: addr.clone() }
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
}

impl AsBytes for IpAddrV4 {
    type Bytes = [u8; 4];

    fn as_bytes(&self) -> &[u8; 4] {
        &self.addr
    }
    fn as_mut_bytes(&mut self) -> &mut [u8; 4] {
        &mut self.addr
    }
    fn from_bytes(addr: &[u8; 4]) -> IpAddrV4 {
        IpAddrV4 { addr: addr.clone() }
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
        IpAddrV6 { scope_id: scope_id, addr: unsafe { let ptr: &[u8; 16] = mem::transmute(&ar); *ptr } }
    }

    pub fn scope_id(&self) -> u32 {
        self.scope_id
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
    fn from_bytes(addr: &[u8; 16]) -> IpAddrV6 {
        IpAddrV6 { scope_id: 0, addr: addr.clone() }
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

pub struct Endpoint<P: Protocol> {
    pro: P,
    ss: libc::sockaddr_storage,
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
                IpAddr::V6(IpAddrV6::from_bytes(unsafe { mem::transmute(&sin6.sin6_addr) }))
            },
            _ => panic!(""),
        }
    }

    pub fn port(&self) -> u16 {
        let sin: &libc::sockaddr_in = unsafe { mem::transmute(&self.ss) };
        u16::from_be(sin.sin_port)
    }

    fn default() -> Endpoint<P> {
        Endpoint {
            pro: P::default(),
            ss: unsafe { mem::zeroed() },
        }
    }

    fn from_v4(addr: &IpAddrV4, port: u16) -> Self {
        let mut ep = Endpoint::default();
        let sin: &mut libc::sockaddr_in = unsafe { mem::transmute(&mut ep.ss) };
        sin.sin_family = ops::FamilyType::Inet as u16;
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
        sin6.sin6_family = ops::FamilyType::Inet6 as u16;
        sin6.sin6_port = port.to_be();
        sin6.sin6_scope_id = addr.scope_id();
        unsafe {
            let src: *const u64 = mem::transmute(addr.as_bytes());
            let dst: *mut u64 = mem::transmute(&mut sin6.sin6_addr);
            ptr::copy(src, dst, 2);
        }
        ep
    }

    fn local_endpoint<'a, S: Socket<'a>>(soc: &mut S) -> io::Result<Self> {
        let mut ep = Endpoint::default();
        try!(ops::local_endpoint(soc, &mut ep));
        Ok(ep)
    }

    fn remote_endpoint<'a, S: Socket<'a>>(soc: &mut S) -> io::Result<Self> {
        let mut ep = Endpoint::default();
        try!(ops::remote_endpoint(soc, &mut ep));
        Ok(ep)
    }
}

impl<P: Protocol> BasicEndpoint<P> for Endpoint<P> {
    fn protocol(&self) -> P {
        self.pro.clone()
    }
}

impl<P: Protocol> AsSockAddr for Endpoint<P> {
    fn socklen(&self) -> ops::NativeSockLenType {
        mem::size_of_val(&self.ss) as ops::NativeSockLenType
    }

    fn as_sockaddr(&self) -> &ops::NativeSockAddrType {
        unsafe { mem::transmute(&self.ss) }
    }

    fn as_mut_sockaddr(&mut self) -> &mut ops::NativeSockAddrType {
        unsafe { mem::transmute(&mut self.ss) }
    }
}

impl<P: Protocol> Eq for Endpoint<P> {
}

impl<P: Protocol> PartialEq for Endpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { libc::memcmp(mem::transmute(self.as_sockaddr()), mem::transmute(other.as_sockaddr()), self.socklen() as usize) == 0 }
    }
}

impl<P: Protocol> Ord for Endpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match unsafe { libc::memcmp(mem::transmute(self.as_sockaddr()), mem::transmute(other.as_sockaddr()), self.socklen() as usize) } {
            0 => cmp::Ordering::Equal,
            x if x < 0 => cmp::Ordering::Less,
            _ => cmp::Ordering::Greater,
        }
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
        match self.addr() {
            IpAddr::V4(addr) => write!(f, "\"{:?}/Ip/{:?}:{:?}\"", self.protocol(), addr, self.port()),
            IpAddr::V6(addr) => write!(f, "\"{:?}/Ip/[{:?}]:{:?}\"", self.protocol(), addr, self.port()),
        }
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

#[derive(Default, Clone, Debug)]
pub struct Tcp;

impl Protocol for Tcp {
    fn family_type<A: AsSockAddr>(&self, sa: &A) -> ops::FamilyType {
        unsafe { mem::transmute( sa.as_sockaddr().sa_family as i8) }
    }
    fn socket_type(&self) -> ops::SocketType {
        ops::SocketType::Stream
    }
    fn protocol_type(&self) -> ops::ProtocolType {
        ops::ProtocolType::Default
    }
}

pub struct TcpStream<'a> {
    io: &'a IoService,
    fd: ops::NativeHandleType,
}

impl<'a> Drop for TcpStream<'a> {
    fn drop(&mut self) {
        let _ = ops::close(self);
    }
}

impl<'a> Socket<'a> for TcpStream<'a> {
    type Endpoint = Endpoint<Tcp>;

    unsafe fn native_handle(&mut self) -> &ops::NativeHandleType {
        &mut self.fd
    }

    fn io_service(&mut self) -> &'a IoService {
        self.io
    }

    fn local_endpoint(&mut self) -> io::Result<Self::Endpoint> {
        Endpoint::local_endpoint(self)
    }
}

impl<'a> StreamSocket<'a> for TcpStream<'a> {
    fn connect(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        let soc = TcpStream { io: io, fd: try!(ops::socket(ep.protocol(), ep)) };
        io.connect(soc, ep)
    }

    fn remote_endpoint(&mut self) -> io::Result<Self::Endpoint> {
        Endpoint::remote_endpoint(self)
    }

    fn receive<B: MutableBuffer>(&mut self, buf: B) -> io::Result<usize> {
        self.io_service().receive(self, buf)
    }

    fn receive_from<B: MutableBuffer>(&mut self, buf: B) -> io::Result<(usize, Self::Endpoint)> {
        let mut ep = Endpoint::default();
        let size = try!(self.io_service().receive_from(self, buf, &mut ep));
        Ok((size, ep))
    }

    fn send<B: Buffer>(&mut self, buf: B) -> io::Result<usize> {
        self.io_service().send(self, buf)
    }

    fn send_to<B: Buffer>(&mut self, buf: B, ep: &Self::Endpoint) -> io::Result<usize> {
        Ok(try!(self.io_service().send_to(self, buf, ep)))
    }
}

pub struct TcpListener<'a> {
    io: &'a IoService,
    fd: ops::NativeHandleType,
}

impl<'a> Drop for TcpListener<'a> {
    fn drop(&mut self) {
        let _ = ops::close(self);
    }
}

impl<'a> Socket<'a> for TcpListener<'a> {
    type Endpoint = Endpoint<Tcp>;

    unsafe fn native_handle(&mut self) -> &ops::NativeHandleType {
        &self.fd
    }

    fn io_service(&mut self) -> &'a IoService {
        self.io
    }

    fn local_endpoint(&mut self) -> io::Result<Self::Endpoint> {
        Endpoint::local_endpoint(self)
    }
}

impl<'a> ListenerSocket<'a> for TcpListener<'a> {
    type StreamSocket = TcpStream<'a>;

    fn listen(io: &'a IoService, ep: Self::Endpoint) -> io::Result<Self> {
        let mut soc = TcpListener { io: io, fd: try!(ops::socket(ep.protocol(), &ep)) };
        try!(ops::set_option(&mut soc, &cmd::ReuseAddr(1)));
        try!(ops::bind(&mut soc, &ep));
        try!(ops::listen(&mut soc));
        Ok(soc)
    }

    fn accept(&mut self) -> io::Result<(Self::StreamSocket, Self::Endpoint)> {
        let mut ep = Endpoint::default();
        let fd = try!(ops::accept(self, &mut ep));
        Ok((TcpStream { io: self.io_service(), fd: fd }, ep))
    }
}

#[derive(Default, Clone, Debug)]
pub struct Udp;

impl Protocol for Udp {
    fn family_type<A: AsSockAddr>(&self, sa: &A) -> ops::FamilyType {
        unsafe { mem::transmute( sa.as_sockaddr().sa_family as i8) }
    }

    fn socket_type(&self) -> ops::SocketType {
        ops::SocketType::Dgram
    }

    fn protocol_type(&self) -> ops::ProtocolType {
        ops::ProtocolType::Default
    }
}

struct UdpSocket<'a> {
    io: &'a IoService,
    fd: ops::NativeHandleType,
}

impl<'a> Drop for UdpSocket<'a> {
    fn drop(&mut self) {
        let _ = ops::close(self);
    }
}

impl<'a> Socket<'a> for UdpSocket<'a> {
    type Endpoint = Endpoint<Udp>;

    unsafe fn native_handle(&mut self) -> &i32 {
        &self.fd
    }

    fn io_service(&mut self) -> &'a IoService {
        self.io
    }

    fn local_endpoint(&mut self) -> io::Result<Self::Endpoint> {
        Endpoint::local_endpoint(self)
    }
}

impl<'a> DgramSocket<'a> for UdpSocket<'a> {
    fn bind(io: &'a IoService, ep: Self::Endpoint) -> io::Result<Self> {
        let mut soc = UdpSocket { io: io, fd: try!(ops::socket(ep.protocol(), &ep)) };
        try!(ops::bind(&mut soc, &ep));
        Ok(soc)
    }

    fn receive<B: MutableBuffer>(&mut self, buf: B) -> io::Result<usize> {
        self.io_service().receive(self, buf)
    }

    fn receive_from<B: MutableBuffer>(&mut self, buf: B) -> io::Result<(usize, Self::Endpoint)> {
        let mut ep = Endpoint::default();
        let size = try!(self.io_service().receive_from(self, buf, &mut ep));
        Ok((size, ep))
    }

    fn send<B: Buffer>(&mut self, buf: B) -> io::Result<usize> {
        self.io_service().send(self, buf)
    }

    fn send_to<B: Buffer>(&mut self, buf: B, ep: &Self::Endpoint) -> io::Result<usize> {
        Ok(try!(self.io_service().send_to(self, buf, ep)))
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
            == IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]));
    assert!(IpAddrV6::new(0,0,0,0,0,0,0,0,100).scope_id() == 100);
    assert!(IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]) <
            IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,17]));
    assert!(IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]) <
            IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,16,00]));
    assert!(IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]) <
            IpAddrV6::from_bytes(&[1,2,3,4,5,6,7,8,9,10,11,12,13,15,00,00]));
}

#[test]
fn test_family_type() {
    let v4: Endpoint<Tcp> = Endpoint::new((IpAddrV4::default(), 0));
    assert!(Tcp::default().family_type(&v4) == ops::FamilyType::Inet);
    assert!(Tcp::default().family_type(&v4) != ops::FamilyType::Inet6);
    assert!(Tcp::default().family_type(&v4) != ops::FamilyType::Local);

    let v6: Endpoint<Tcp> = Endpoint::new((IpAddrV6::default(), 0));
    assert!(Tcp::default().family_type(&v6) != ops::FamilyType::Inet);
    assert!(Tcp::default().family_type(&v6) == ops::FamilyType::Inet6);
    assert!(Tcp::default().family_type(&v6) != ops::FamilyType::Local);
}

#[test]
fn test_socket_type() {
    assert!(Tcp::default().socket_type() == ops::SocketType::Stream);
    assert!(Tcp::default().socket_type() != ops::SocketType::Dgram);
    assert!(Udp::default().socket_type() != ops::SocketType::Stream);
    assert!(Udp::default().socket_type() == ops::SocketType::Dgram);
}

#[test]
fn test_endpoint_v4() {
    let ep: Endpoint<Udp> = Endpoint::new((IpAddrV4::new(1,2,3,4), 10));
    assert!(ep.is_v4());
    assert!(ep.addr() == IpAddr::V4(IpAddrV4::new(1,2,3,4)));
    assert!(ep.port() == 10);
    assert!(!ep.is_v6());
}

#[test]
fn test_endpoint_v6() {
    let ep: Endpoint<Tcp> = Endpoint::new((IpAddrV6::new(1,2,3,4,5,6,7,8,0), 10));
    assert!(ep.is_v6());
    assert!(ep.addr() == IpAddr::V6(IpAddrV6::new(1,2,3,4,5,6,7,8,0)));
    assert!(ep.port() == 10);
    assert!(!ep.is_v4());
}

#[test]
fn test_endpoint_cmp() {
    let a: Endpoint<Tcp> = Endpoint::new((IpAddrV6::new(1,2,3,4,5,6,7,8,0), 10));
    let b: Endpoint<Tcp> = Endpoint::new((IpAddrV6::new(1,2,3,4,5,6,7,8,1), 10));
    let c: Endpoint<Tcp> = Endpoint::new((IpAddrV6::new(1,2,3,4,5,6,7,8,0), 11));
    assert!(a == a && b == b && c == c);
    assert!(a != b && b != c);
    assert!(a < b);
    assert!(b < c);
}
