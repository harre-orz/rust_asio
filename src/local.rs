use super::{IoService, Protocol, AsSockAddr, Endpoint as BasicEndpoint, Socket, StreamSocket, ListenerSocket, DgramSocket, SeqPacketSocket, Buffer, MutableBuffer};
use super::ops;
use std::io;
use std::fmt;
use std::mem;
use std::cmp;
use libc;

pub struct Endpoint<P: Protocol> {
    pro: P,
    sun: libc::sockaddr_un,
}

const UNIX_PATH_MAX: usize = 108;
impl<P: Protocol> Endpoint<P> {
    pub fn new(path: &str) -> Endpoint<P> {
        let mut ep = Endpoint::default();
        ep.sun.sun_family = ops::FamilyType::Local as u16;
        for (a, c) in ep.sun.sun_path[0..UNIX_PATH_MAX-1].iter_mut().zip(path.chars()) {
            *a = c as i8;
        }
        ep
    }

    pub fn path(&self) -> String {
        let mut s = String::new();
        for c in self.sun.sun_path.iter() {
            if *c == 0 { break; }
            s.push((*c as u8) as char);
        }
        s
    }

    fn default() -> Endpoint<P> {
        Endpoint {
            pro: P::default(),
            sun: unsafe { mem::zeroed() },
        }
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
        mem::size_of_val(&self.sun) as ops::NativeSockLenType
    }

    fn as_sockaddr(&self) -> &ops::NativeSockAddrType {
        unsafe { mem::transmute(&self.sun) }
    }

    fn as_mut_sockaddr(&mut self) -> &mut ops::NativeSockAddrType {
        unsafe { mem::transmute(&mut self.sun) }
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
        write!(f, "{}", self.path())
    }
}

impl<P: Protocol> fmt::Debug for Endpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}/Local/{:?}", self.protocol(), self.path())
    }
}

#[derive(Default, Clone, Debug)]
pub struct Stream;

impl Protocol for Stream {
    fn family_type<E: BasicEndpoint<Stream>>(&self, _: &E) -> ops::FamilyType {
        ops::FamilyType::Local
    }

    fn socket_type<E: BasicEndpoint<Stream>>(&self, _: &E) -> ops::SocketType {
        ops::SocketType::Stream
    }

    fn protocol_type<E: BasicEndpoint<Stream>>(&self, _: &E) -> ops::ProtocolType {
        ops::ProtocolType::Default
    }
}

pub struct LocalStream<'a> {
    io: &'a IoService,
    fd: ops::NativeHandleType,
}

impl<'a> Drop for LocalStream<'a> {
    fn drop(&mut self) {
        let _ = ops::close(self);
    }
}

impl<'a> Socket<'a> for LocalStream<'a> {
    type Endpoint = Endpoint<Stream>;

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

impl<'a> StreamSocket<'a> for LocalStream<'a> {
    fn connect(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        let mut soc = LocalStream { io: io, fd: try!(ops::socket(ep.protocol(), ep)) };
        try!(ops::connect(&mut soc, ep));
        Ok(soc)
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

pub struct LocalListener<'a> {
    io: &'a IoService,
    fd: ops::NativeHandleType,
}

impl<'a> Drop for LocalListener<'a> {
    fn drop(&mut self) {
        let _ = ops::close(self);
    }
}

impl<'a> Socket<'a> for LocalListener<'a> {
    type Endpoint = Endpoint<Stream>;

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

impl<'a> ListenerSocket<'a> for LocalListener<'a> {
    type StreamSocket = LocalStream<'a>;

    fn listen(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        let mut soc = LocalListener { io: io, fd: try!(ops::socket(ep.protocol(), ep)) };
        try!(ops::bind(&mut soc, ep));
        try!(ops::listen(&mut soc));
        Ok(soc)
    }

    fn accept(&mut self) -> io::Result<(Self::StreamSocket, Self::Endpoint)> {
        let mut ep = Endpoint::default();
        let fd = try!(ops::accept(self, &mut ep));
        Ok((LocalStream { io: self.io_service(), fd: fd }, ep))
    }
}

#[derive(Default, Clone, Debug)]
pub struct Dgram;

impl Protocol for Dgram {
    fn family_type<E: BasicEndpoint<Dgram>>(&self, _: &E) -> ops::FamilyType {
        ops::FamilyType::Local
    }

    fn socket_type<E: BasicEndpoint<Dgram>>(&self, _: &E) -> ops::SocketType {
        ops::SocketType::Dgram
    }

    fn protocol_type<E: BasicEndpoint<Dgram>>(&self, _: &E) -> ops::ProtocolType {
        ops::ProtocolType::Default
    }
}

pub struct LocalDgram<'a> {
    io: &'a IoService,
    fd: ops::NativeHandleType,
}

impl<'a> Drop for LocalDgram<'a> {
    fn drop(&mut self) {
        let _ = ops::close(self);
    }
}

impl<'a> Socket<'a> for LocalDgram<'a> {
    type Endpoint = Endpoint<Dgram>;

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

impl<'a>  DgramSocket<'a> for LocalDgram<'a> {
    fn bind(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        let mut soc = LocalDgram { io: io, fd: try!(ops::socket(ep.protocol(), ep)) };
        try!(ops::bind(&mut soc, ep));
        Ok(soc)
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
