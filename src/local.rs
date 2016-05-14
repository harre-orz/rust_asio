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
use libc;

#[derive(Clone)]
pub struct Endpoint<P: Protocol> {
    sun: libc::sockaddr_un,
    marker: PhantomData<P>,
}

const UNIX_PATH_MAX: usize = 108;
impl<P: Protocol> Endpoint<P> {
    pub fn new<T: AsRef<str>>(path: T) -> Endpoint<P> {
        let mut ep = Endpoint::default();
        ep.sun.sun_family = libc::AF_UNIX as u16;
        for (a, c) in ep.sun.sun_path[0..UNIX_PATH_MAX-1].iter_mut().zip(path.as_ref().chars()) { *a = c as i8; }
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
            sun: unsafe { mem::zeroed() },
            marker: PhantomData,
        }
    }
}

impl<P: Protocol> AsSockAddr for Endpoint<P> {
    fn socklen(&self) -> NativeSockLenType {
        mem::size_of_val(&self.sun) as NativeSockLenType
    }

    fn as_sockaddr(&self) -> &NativeSockAddrType {
        unsafe { mem::transmute(&self.sun) }
    }

    fn as_mut_sockaddr(&mut self) -> &mut NativeSockAddrType {
        unsafe { mem::transmute(&mut self.sun) }
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
        write!(f, "{}", self.path())
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Stream;

impl Protocol for Stream {
    fn family_type(&self) -> i32 {
        libc::AF_UNIX
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_STREAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl BasicEndpoint<Stream> for Endpoint<Stream> {
    fn protocol(&self) -> Stream {
        Stream
    }
}

pub type StreamEndpoint = Endpoint<Stream>;

pub struct LocalStream<'a> {
    _impl: BasicSocket<'a, Stream>,
}

impl<'a> IoObject<'a> for LocalStream<'a> {
    fn io_service(&self) -> &'a IoService {
        self._impl.io_service()
    }
}

impl<'a> SocketBase<Stream> for LocalStream<'a> {
    type Endpoint = Endpoint<Stream>;

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

impl<'a> StreamSocket<'a, Stream> for LocalStream<'a> {
    fn connect(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        Ok(LocalStream { _impl: try!(BasicSocket::connect(io, ep)) })
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

impl<'a> ReadWrite<'a> for LocalStream<'a> {
    fn read_some<B: MutableBuffer>(&self, buf: B) -> io::Result<usize> {
        self._impl.recv(buf, 0)
    }

    fn write_some<B: Buffer>(&self, buf: B) -> io::Result<usize> {
        self._impl.send(buf, 0)
    }
}

pub struct LocalListener<'a> {
    _impl: BasicSocket<'a, Stream>,
}

impl<'a> IoObject<'a> for LocalListener<'a> {
    fn io_service(&self) -> &'a IoService {
        self._impl.io_service()
    }
}

impl<'a> SocketBase<Stream> for LocalListener<'a> {
    type Endpoint = Endpoint<Stream>;

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

impl<'a> ListenerSocket<'a, Stream> for LocalListener<'a> {
    type Socket = LocalStream<'a>;

    fn listen(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        Ok(LocalListener { _impl: try!(BasicSocket::listen(io, ep)) })
    }

    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)> {
        let _impl = try!(self._impl.accept(Endpoint::default()));
        Ok((LocalStream { _impl: _impl.0 }, _impl.1))
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Dgram;

impl Protocol for Dgram {
    fn family_type(&self) -> i32 {
        libc::AF_UNIX
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_DGRAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl BasicEndpoint<Dgram> for Endpoint<Dgram> {
    fn protocol(&self) -> Dgram {
        Dgram
    }
}

pub type DgramEndpoint = Endpoint<Dgram>;

pub struct LocalDgram<'a> {
    _impl: BasicSocket<'a, Dgram>,
}

impl<'a> IoObject<'a> for LocalDgram<'a> {
    fn io_service(&self) -> &'a IoService {
        self._impl.io_service()
    }
}

impl<'a> SocketBase<Dgram> for LocalDgram<'a> {
    type Endpoint = Endpoint<Dgram>;

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

impl<'a> Socket<'a, Dgram> for LocalDgram<'a> {
    fn bind(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        Ok(LocalDgram { _impl: try!(BasicSocket::bind(io, ep)) })
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
pub struct SeqPacket;

const SOCK_SEQPACKET: i32 = 5;
impl Protocol for SeqPacket {
    fn family_type(&self) -> i32 {
        libc::AF_UNIX
    }

    fn socket_type(&self) -> i32 {
        SOCK_SEQPACKET
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl BasicEndpoint<SeqPacket> for Endpoint<SeqPacket> {
    fn protocol(&self) -> SeqPacket {
        SeqPacket
    }
}

pub type SeqPacketEndpoint = Endpoint<SeqPacket>;

pub struct LocalSeqSocket<'a> {
    _impl: BasicSocket<'a, SeqPacket>,
}

impl<'a> IoObject<'a> for LocalSeqSocket<'a> {
    fn io_service(&self) -> &'a IoService {
        self._impl.io_service()
    }
}

impl<'a> SocketBase<SeqPacket> for LocalSeqSocket<'a> {
    type Endpoint = Endpoint<SeqPacket>;

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

impl<'a> StreamSocket<'a, SeqPacket> for LocalSeqSocket<'a> {
    fn connect(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        Ok(LocalSeqSocket { _impl: try!(BasicSocket::connect(io, ep)) })
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

pub struct LocalSeqListener<'a> {
    _impl: BasicSocket<'a, SeqPacket>,
}

impl<'a> IoObject<'a> for LocalSeqListener<'a> {
    fn io_service(&self) -> &'a IoService {
        self._impl.io_service()
    }
}

impl<'a> SocketBase<SeqPacket> for LocalSeqListener<'a> {
    type Endpoint = Endpoint<SeqPacket>;

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

impl<'a> ListenerSocket<'a, SeqPacket> for LocalSeqListener<'a> {
    type Socket = LocalSeqSocket<'a>;

    fn listen(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self> {
        Ok(LocalSeqListener { _impl: try!(BasicSocket::listen(io, ep)) })
    }

    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)> {
        let _impl = try!(self._impl.accept(Endpoint::default()));
        Ok((LocalSeqSocket { _impl: _impl.0 }, _impl.1))
    }
}

#[test]
fn test_endpoint() {
    let ep: Endpoint<Stream> = Endpoint::new("hello");
    assert!(ep.path() == "hello");
}

#[test]
fn test_stream() {
    assert!(Stream == Stream);
}

#[test]
fn test_dgram() {
    assert!(Dgram == Dgram);
}

#[test]
fn test_seqpacket() {
    assert!(SeqPacket == SeqPacket);
}
