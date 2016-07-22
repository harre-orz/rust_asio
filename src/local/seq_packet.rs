use std::io;
use std::mem;
use {IoObject, Strand, Protocol, Endpoint, SeqPacketSocket, SocketListener};
use super::LocalEndpoint;
use ops;
use ops::{AF_LOCAL, SOCK_SEQPACKET};
use ops::async::*;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalSeqPacket;

impl Protocol for LocalSeqPacket {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        SOCK_SEQPACKET
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl Endpoint<LocalSeqPacket> for LocalEndpoint<LocalSeqPacket> {
    fn protocol(&self) -> LocalSeqPacket {
        LocalSeqPacket
    }
}

impl SeqPacketSocket<LocalSeqPacket> {
    pub fn new<T: IoObject>(io: &T) -> io::Result<SeqPacketSocket<LocalSeqPacket>> {
        let soc = try!(ops::socket(&LocalSeqPacket));
        Ok(Self::_new(io, LocalSeqPacket, soc))
    }
}

impl SocketListener<LocalSeqPacket> {
    pub fn new<T: IoObject>(io: &T) -> io::Result<SocketListener<LocalSeqPacket>> {
        let soc = try!(ops::socket(&LocalSeqPacket));
        Ok(Self::_new(io, LocalSeqPacket, soc))
    }

    pub fn accept(&self) -> io::Result<(LocalSeqPacketSocket, LocalSeqPacketEndpoint)> {
        let (soc, ep) = try!(syncd_accept(self, unsafe { mem::uninitialized() }));
        Ok((LocalSeqPacketSocket::_new(self.io_service(), self.pro.clone(), soc), ep))
    }

    pub unsafe fn async_accept<F, T>(&self, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<(LocalSeqPacketSocket, LocalSeqPacketEndpoint)>) + Send + 'static,
              T: 'static {
        let pro = self.pro.clone();
        async_accept(self, unsafe { mem::uninitialized() }, move |obj, res| {
            match res {
                Ok((soc, ep)) => {
                    let soc = LocalSeqPacketSocket::_new(&obj, pro, soc);
                    callback(obj, Ok((soc, ep)))
                }
                Err(err) => callback(obj, Err(err))
            }
        }, strand)
    }
}

pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

pub type LocalSeqPacketSocket = SeqPacketSocket<LocalSeqPacket>;

pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket>;

#[test]
fn test_seq_packet() {
    assert!(LocalSeqPacket == LocalSeqPacket);
}
