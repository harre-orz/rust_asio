#![feature(libc)]
extern crate libc;
use std::io;
use std::fmt::{Display, Debug};

macro_rules! libc_try {
    ($expr:expr) => (match unsafe { $expr } {
        -1 => return Err(io::Error::last_os_error()),
        rc => rc,
    })
}

pub type IoService = srv::TakoIoService;

pub trait IoObject<'a> {
    fn io_service(&self) -> &'a IoService;
}

pub enum Shutdown {
    Read, Write, Both,
}

pub trait AsBytes {
    type Bytes;
    fn as_bytes(&self) -> &Self::Bytes;
    fn as_mut_bytes(&mut self) -> &mut Self::Bytes;
}

pub trait Protocol : Default + Clone + Debug {
    fn family_type<E: Endpoint<Self>>(&self, ep: &E) -> ops::FamilyType;
    fn socket_type<E: Endpoint<Self>>(&self, ep: &E) -> ops::SocketType;
    fn protocol_type<E: Endpoint<Self>>(&self, ep: &E) -> ops::ProtocolType;
}

pub trait Endpoint<P: Protocol> : Eq + PartialEq + Ord + PartialOrd + Display + Debug {
    fn protocol(&self) -> P;
    fn as_sockaddr(&self) -> &ops::NativeSockAddrType;
    fn as_mut_sockaddr(&mut self) -> &mut ops::NativeSockAddrType;
    fn socklen(&self) -> ops::NativeSockLenType;
}

pub trait Resolver<P: Protocol> {
    type Iter;
    fn resolve(&mut self, host: &str, port: &str) -> Self::Iter;
}

pub trait Socket<'a> : IoObject<'a> + Sized + Drop {
    type Endpoint;
    unsafe fn native_handle(&mut self) -> &ops::NativeHandleType;
    fn local_endpoint(&mut self) -> io::Result<Self::Endpoint>;
    fn available(&mut self) -> io::Result<usize> {
        let mut cmd = cmd::Available(0);
        try!(self.io_control(&mut cmd));
        Ok((cmd.0 as usize))
    }
    fn get_nonblocking(&mut self) -> io::Result<bool> {
        ops::get_nonblocking(self)
    }
    fn set_nonblocking(&mut self, on: bool) -> io::Result<()> {
        ops::set_nonblocking(self, on)
    }
    fn io_control<T: IoControlCommand>(&mut self, cmd: &mut T) -> io::Result<()> {
        ops::io_control(self, cmd)
    }
    fn get_option<T: GetOptionCommand>(&mut self, cmd: &mut T) -> io::Result<()> {
        ops::get_option(self, cmd)
    }
    fn set_option<T: SetOptionCommand>(&mut self, cmd: &T) -> io::Result<()> {
        ops::set_option(self, cmd)
    }
}

pub trait StreamSocket<'a> : Socket<'a> {
    fn connect(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn remote_endpoint(&mut self) -> io::Result<Self::Endpoint>;
    fn shutdown(&mut self, how: Shutdown) -> io::Result<()> {
        ops::shutdown(self, how)
    }

    fn receive<B: MutableBuffer>(&mut self, buf: B) -> io::Result<usize>;
    fn receive_from<B: MutableBuffer>(&mut self, buf: B) -> io::Result<(usize, Self::Endpoint)>;
    fn send<B: Buffer>(&mut self, buf: B) -> io::Result<usize>;
    fn send_to<B: Buffer>(&mut self, buf: B, ep: &Self::Endpoint) -> io::Result<usize>;
}

pub trait ListenerSocket<'a> : Socket<'a> {
    type StreamSocket;
    fn listen(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn accept(&mut self) -> io::Result<(Self::StreamSocket, Self::Endpoint)>;
}

pub trait DgramSocket<'a> : Socket<'a> {
    fn bind(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn remote_endpoint(&mut self) -> io::Result<Self::Endpoint>;
    fn receive<B: MutableBuffer>(&mut self, buf: B) -> io::Result<usize>;
    fn receive_from<B: MutableBuffer>(&mut self, buf: B) -> io::Result<(usize, Self::Endpoint)>;
    fn send<B: Buffer>(&mut self, buf: B) -> io::Result<usize>;
    fn send_to<B: Buffer>(&mut self, buf: B, ep: &Self::Endpoint) -> io::Result<usize>;
}

pub trait SeqPacketSocket<'a> : Socket<'a> {
    fn connect(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
}

pub trait RawSocket<'a> : Socket<'a> {
    fn bind(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn remote_endpoint(&mut self) -> io::Result<Self::Endpoint>;
    fn receive<B: MutableBuffer>(&mut self, buf: B) -> io::Result<usize>;
    fn receive_from<B: MutableBuffer>(&mut self, buf: B) -> io::Result<(usize, Self::Endpoint)>;
    fn send<B: Buffer>(&mut self, buf: B) -> io::Result<usize>;
    fn send_to<B: Buffer>(&mut self, buf: B, ep: &Self::Endpoint) -> io::Result<usize>;
}

pub trait Buffer {
    fn buffer_size(&self) -> usize;
    fn as_buffer(&self) -> &[u8];
}

pub trait MutableBuffer : Buffer {
    fn as_mut_buffer(&mut self) -> &mut [u8];
}

pub trait IoControlCommand {
    type Data;
    fn name(&self) -> i32;
    fn data(&mut self) -> &mut Self::Data;
}

pub trait OptionCommand {
    type Data;
    fn level(&self) -> i32;
    fn name(&self) -> i32;
}

pub trait GetOptionCommand : OptionCommand {
    fn resize(&mut self, s: usize);
    fn data_mut(&mut self) -> &mut Self::Data;
}

pub trait SetOptionCommand : OptionCommand {
    fn size(&self) -> usize;
    fn data(&self) -> &Self::Data;
}

mod ops;

mod str;

mod buf;

mod cmd;

mod srv;

pub mod ip;

pub mod local;

mod soc;
