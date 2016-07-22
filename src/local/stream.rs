use std::io;
use std::mem;
use {IoObject, Strand, Protocol, Endpoint, StreamSocket, SocketListener};
use super::LocalEndpoint;
use ops;
use ops::{AF_LOCAL, SOCK_STREAM};
use ops::async::*;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalStream;

impl Protocol for LocalStream {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl Endpoint<LocalStream> for LocalEndpoint<LocalStream> {
    fn protocol(&self) -> LocalStream {
        LocalStream
    }
}

impl StreamSocket<LocalStream> {
    pub fn new<T: IoObject>(io: &T) -> io::Result<StreamSocket<LocalStream>> {
        let soc = try!(ops::socket(&LocalStream));
        Ok(Self::_new(io, LocalStream, soc))
    }
}

impl SocketListener<LocalStream> {
    pub fn new<T: IoObject>(io: &T) -> io::Result<SocketListener<LocalStream>> {
        let soc = try!(ops::socket(&LocalStream));
        Ok(Self::_new(io, LocalStream, soc))
    }

    pub fn accept(&self) -> io::Result<(LocalStreamSocket, LocalStreamEndpoint)> {
        let (soc, ep) = try!(syncd_accept(self, unsafe { mem::uninitialized() }));
        Ok((LocalStreamSocket::_new(self, self.pro.clone(), soc), ep))
    }

    pub unsafe fn async_accept<F, T>(&self, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<(LocalStreamSocket, LocalStreamEndpoint)>) + Send + 'static,
              T: 'static {
        let pro = self.pro.clone();
        async_accept(self, unsafe { mem::uninitialized() }, move |obj, res| {
            match res {
                Ok((soc, ep)) => {
                    let soc = LocalStreamSocket::_new(&obj, pro, soc);
                    callback(obj, Ok((soc, ep)))
                }
                Err(err) => callback(obj, Err(err))
            }
        }, strand)
    }
}

pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

pub type LocalStreamSocket = StreamSocket<LocalStream>;

pub type LocalStreamListener = SocketListener<LocalStream>;

#[test]
fn test_stream() {
    assert!(LocalStream == LocalStream);
}
