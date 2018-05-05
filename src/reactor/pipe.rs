use ffi::{AsRawFd, RawFd, close, SystemError, pipe};
use core::{Handle, Reactor};

use std::mem;
use libc;

pub struct PipeIntr {
    rfd: Handle,
    wfd: RawFd,
}

impl PipeIntr {
    pub fn new() -> Result<Self, SystemError> {
        let (rfd, wfd) = pipe()?;
        Ok(PipeIntr {
            rfd: Handle::intr(rfd),
            wfd: wfd,
        })
    }

    pub fn startup(&self, reactor: &Reactor) {
        reactor.register_intr(&self.rfd);
    }

    pub fn cleanup(&self, reactor: &Reactor) {
        reactor.deregister_intr(&self.rfd)
    }

    pub fn interrupt(&self) {
        unsafe {
            let buf: [u8; 1] = mem::uninitialized();
            libc::write(self.wfd, buf.as_ptr() as *const libc::c_void, buf.len());
        }
    }
}

impl Drop for PipeIntr {
    fn drop(&mut self) {
        close(self.rfd.as_raw_fd());
        close(self.wfd);
    }
}
