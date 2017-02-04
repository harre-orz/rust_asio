use core::{Reactor, IntrFd};

use std::io;

pub type Interrupter = NullInterrupter;

pub struct NullInterrupter {
    _intrfd: IntrFd,
}

impl NullInterrupter {
    pub fn new() -> io::Result<Self> {
        Ok(NullInterrupter {
            _intrfd: IntrFd::new::<Self>(0),
        })
    }

    pub fn startup(&self, ctx: &Reactor) {
    }

    pub fn cleanup(&self, ctx: &Reactor) {
    }

    pub fn interrupt(&self) {
    }
}
