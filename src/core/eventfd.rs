use ffi::{AsRawFd, close, write, SystemError};
use core::{Handle, Reactor};

use libc::{eventfd, EFD_CLOEXEC, EFD_NONBLOCK};

pub struct EventFdIntr {
   efd: Handle,
}

impl EventFdIntr {
    pub fn new() -> Result<Self, SystemError> {
        match unsafe { eventfd(0, EFD_CLOEXEC | EFD_NONBLOCK) } {
            -1 => Err(SystemError::last_error()),
            fd => Ok(EventFdIntr {
                efd: Handle::intr(fd),
            })
        }
    }

    pub fn startup(&self, reactor: &Reactor) {
        reactor.register_intr(&self.efd);
    }

    pub fn cleanup(&self, reactor: &Reactor) {
        reactor.deregister_intr(&self.efd)
    }

    pub fn interrupt(&self) {
        let buf = [1,0,0,0,0,0,0,0];
        write(&self.efd, &buf).unwrap();
    }
}

impl Drop for EventFdIntr {
    fn drop(&mut self) {
        close(self.efd.as_raw_fd());
    }
}
