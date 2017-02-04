use core::{Reactor, IntrFd};
use ffi::write;

use std::io;
use libc::{eventfd, EFD_CLOEXEC, EFD_NONBLOCK};

pub struct EventFdInterrupter {
    efd: IntrFd,
}

impl EventFdInterrupter {
    pub fn new() -> io::Result<Self> {
        let efd = libc_try!(eventfd(0, EFD_CLOEXEC | EFD_NONBLOCK));
        Ok(EventFdInterrupter {
            efd: IntrFd::new::<Self>(efd),
        })
    }

    pub fn startup(&self, ctx: &Reactor) {
        ctx.register_intr_fd(&self.efd)
    }

    pub fn cleanup(&self, ctx: &Reactor) {
        ctx.deregister_intr_fd(&self.efd)
    }

    pub fn interrupt(&self) {
        let buf = [1,0,0,0,0,0,0,0];
        libc_ign!(write(&self.efd, &buf));
    }
}
