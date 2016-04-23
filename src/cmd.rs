use super::*;
use std::mem;
use libc;

pub struct Available(pub i32);
impl IoControlCommand for Available {
    type Data = i32;

    fn name(&self) -> i32 {
        libc::FIONREAD as i32
    }

    fn data(&mut self) -> &mut i32 {
        &mut self.0
    }
}

pub struct ReuseAddr(pub i32);
impl OptionCommand for ReuseAddr {
    type Data = i32;

    fn level(&self) -> i32 {
        libc::SOL_SOCKET
    }

    fn name(&self) -> i32 {
        libc::SO_REUSEADDR
    }
}

impl SetOptionCommand for ReuseAddr {
    fn size(&self) -> usize {
        mem::size_of::<Self::Data>()
    }

    fn data(&self) -> &Self::Data {
        &self.0
    }
}
