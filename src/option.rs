use std::mem;
use super::*;
use libc;

pub struct Available(pub i32);
impl IoControl for Available {
    type Data = i32;

    fn name(&self) -> i32 {
        libc::FIONREAD as i32
    }

    fn data(&mut self) -> &mut i32 {
        &mut self.0
    }
}

pub struct ReuseAddr(pub i32);

impl Default for ReuseAddr {
    fn default() -> Self {
        ReuseAddr(0)
    }
}

impl SocketOption for ReuseAddr {
    type Data = i32;

    fn level(&self) -> i32 {
        libc::SOL_SOCKET
    }

    fn name(&self) -> i32 {
        libc::SO_REUSEADDR
    }
}

impl SetSocketOption for ReuseAddr {
    fn size(&self) -> usize {
        mem::size_of::<Self::Data>()
    }

    fn data(&self) -> &Self::Data {
        &self.0
    }
}
