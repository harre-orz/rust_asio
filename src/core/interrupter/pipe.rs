use core::{Reactor, IntrFd};
use ffi::{FD_CLOEXEC, pipe, write, setnonblock};

use std::io;
use std::mem;

pub struct PipeInterrupter {
    pipe_rfd: IntrFd,
    pipe_wfd: IntrFd,
}

impl PipeInterrupter {
    pub fn new() -> io::Result<Self> {
        let (rfd, wfd) = try!(pipe(FD_CLOEXEC));
        let rfd = IntrFd::new::<Self>(rfd);
        let wfd = IntrFd::new::<Self>(wfd);
        try!(setnonblock(&rfd, true));
        try!(setnonblock(&wfd, true));
        Ok(PipeInterrupter {
            pipe_rfd: rfd,
            pipe_wfd: wfd,
        })
    }

    pub fn startup(&self, ctx: &Reactor) {
        ctx.register_intr_fd(&self.pipe_rfd)
    }

    pub fn cleanup(&self, ctx: &Reactor) {
        ctx.deregister_intr_fd(&self.pipe_rfd)
    }

    pub fn interrupt(&self) {
        let buf: [u8; 1] = unsafe { mem::uninitialized() };
        libc_ign!(write(&self.pipe_wfd, &buf));
    }
}
