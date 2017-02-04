use ffi::{RawFd, AsRawFd, INVALID_SOCKET, FD_SETSIZE};

use std::mem;
use libc::{fd_set, FD_ZERO, FD_SET, FD_ISSET};

pub struct PosixFdSet {
    fds: fd_set,
    max_fd: RawFd,
}

impl PosixFdSet {
    pub fn new() -> PosixFdSet {
        let mut fds: fd_set = unsafe { mem::uninitialized() };
        unsafe { FD_ZERO(&mut fds); }
        PosixFdSet { fds: fds, max_fd: INVALID_SOCKET }
    }

    pub fn reset(&mut self) {
        unsafe { FD_ZERO(&mut self.fds); }
        self.max_fd = INVALID_SOCKET;
    }

    pub fn set<T>(&mut self, t: &T) -> bool
        where T: AsRawFd,
    {
        let fd = t.as_raw_fd();
        if fd < FD_SETSIZE as _ {
            unsafe { FD_SET(fd,& mut self.fds); }
            if self.max_fd == INVALID_SOCKET || self.max_fd < fd {
                self.max_fd = fd;
            }
            true
        } else {
            false
        }
    }

    pub fn is_set<T>(&self, t: &T) -> bool
        where T: AsRawFd,
    {
        let fds = &self.fds as *const _ as *mut _;
        unsafe { FD_ISSET(t.as_raw_fd(), &mut *fds) }
    }

    pub fn max_fd(&self) -> RawFd {
        self.max_fd
    }
}
