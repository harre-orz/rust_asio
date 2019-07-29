//

use super::{
    LocalDgram, LocalDgramSocket, LocalSeqPacket, LocalSeqPacketSocket, LocalStream,
    LocalStreamSocket,
};
use executor::IoContext;
use socket::socketpair;
use std::io;

pub struct LocalPair<P> {
    _pro: P,
}

impl<P> LocalPair<P> {
    pub fn new(pro: P) -> Self {
        LocalPair { _pro: pro }
    }
}

impl LocalPair<LocalDgram> {
    pub fn connect(&self, ctx: &IoContext) -> io::Result<(LocalDgramSocket, LocalDgramSocket)> {
        Ok(socketpair(ctx, LocalDgram, LocalDgram)?)
    }
}

impl LocalPair<LocalStream> {
    pub fn connect(&self, ctx: &IoContext) -> io::Result<(LocalStreamSocket, LocalStreamSocket)> {
        Ok(socketpair(ctx, LocalStream, LocalStream)?)
    }
}

impl LocalPair<LocalSeqPacket> {
    pub fn connect(
        &self,
        ctx: &IoContext,
    ) -> io::Result<(LocalSeqPacketSocket, LocalSeqPacketSocket)> {
        Ok(socketpair(ctx, LocalSeqPacket, LocalSeqPacket)?)
    }
}
