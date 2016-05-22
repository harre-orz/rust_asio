use std::mem;
use socket::{IoControl, SocketOption, GetSocketOption, SetSocketOption};
use ops::*;

#[derive(Default, Clone)]
pub struct Available(pub i32);

impl IoControl for Available {
    type Data = i32;

    fn name(&self) -> i32 {
        FIONREAD as i32
    }

    fn data(&mut self) -> &mut i32 {
        &mut self.0
    }
}

#[derive(Default, Clone)]
pub struct ReuseAddr(pub i32);

impl SocketOption for ReuseAddr {
    type Data = i32;

    fn level(&self) -> i32 {
        SOL_SOCKET
    }

    fn name(&self) -> i32 {
        SO_REUSEADDR
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
