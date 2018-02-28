use super::{Kevent, KqueueReactor};
use ffi::{AsRawFd, SystemError, pipe, write, close};

use std::mem;


struct PipeIntrImpl {
    rfd: Kevent,
    wfd: Kevent,
}

impl Drop for PipeIntrImpl {
    fn drop(&mut self) {
        close(self.rfd.as_raw_fd());
        close(self.wfd.as_raw_fd());
    }
}

pub struct PipeIntr(Box<PipeIntrImpl>);

impl PipeIntr {
    pub fn new() -> Result<Self, SystemError> {
        let (rfd, wfd) = pipe()?;
        Ok(PipeIntr(Box::new(PipeIntrImpl {
            rfd: Kevent::intr(rfd),
            wfd: Kevent::intr(wfd),
        })))
    }

    pub fn startup(&self, reactor: &KqueueReactor) {
        reactor.register_intr(&self.0.rfd);
    }

    pub fn cleanup(&self, reactor: &KqueueReactor) {
        reactor.deregister_intr(&self.0.rfd)
    }

    pub fn interrupt(&self) {
        let buf: [u8; 1] = unsafe { mem::uninitialized() };
        let _ = write(&self.0.wfd, &buf);
    }
}
