use super::*;
use std::io;
use std::fmt::{Display,};

pub trait Protocol : Clone + Eq + PartialEq {
    fn family_type(&self) -> i32;
    fn socket_type(&self) -> i32;
    fn protocol_type(&self) -> i32;
}

pub trait Endpoint<P: Protocol> : Clone + Eq + PartialEq + Ord + PartialOrd + Display {
    fn protocol(&self) -> P;
}

pub trait ResolveQuery<'i, P: Protocol> {
    type Iter : Iterator;
    fn query(self, pro: P) -> io::Result<Self::Iter>;
}

pub trait Resolver<'a, P: Protocol> : IoObject<'a> {
    fn new(io: &'a IoService) -> io::Result<Self>;
    fn resolve<'i, T: ResolveQuery<'i, P>>(&self, t: T) -> io::Result<T::Iter>;
}

pub trait SocketBase<P: Protocol> {
    type Endpoint : Endpoint<P>;
    unsafe fn native_handle(&self) -> &NativeHandleType;
    fn local_endpoint(&self) -> io::Result<Self::Endpoint>;
    fn io_control<T: IoControl>(&self, cmd: &mut T) -> io::Result<()>;
    fn get_socket<T: GetSocketOption>(&self) -> io::Result<T>;
    fn set_socket<T: SetSocketOption>(&self, cmd: &T) -> io::Result<()>;
    fn get_non_blocking(&self) -> io::Result<bool>;
    fn set_non_blocking(&self, on: bool) -> io::Result<()>;
}

pub trait Socket<'a, P: Protocol> : IoObject<'a> + SocketBase<P> {
    fn bind(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()>;
    fn remote_endpoint(&self) -> io::Result<Self::Endpoint>;
    fn available(&self) -> io::Result<usize>;
    fn recv<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<usize>;
    fn send<B: Buffer>(&self, buf: B, flags: i32) -> io::Result<usize>;
    fn recv_from<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<(usize, Self::Endpoint)>;
    fn send_to<B: Buffer>(&self, buf: B, flags: i32, ep: &Self::Endpoint) -> io::Result<usize>;
}

pub trait StreamSocket<'a, P: Protocol> : IoObject<'a> + SocketBase<P> {
    fn connect(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn shutdown(&self, how: Shutdown) -> io::Result<()>;
    fn remote_endpoint(&self) -> io::Result<Self::Endpoint>;
    fn available(&self) -> io::Result<usize>;
    fn recv<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<usize>;
    fn send<B: Buffer>(&self, buf: B, flags: i32) -> io::Result<usize>;
}

pub trait ListenerSocket<'a, P: Protocol> : IoObject<'a> + SocketBase<P> {
    type Socket : IoObject<'a> + SocketBase<P>;
    fn listen(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)>;
}
