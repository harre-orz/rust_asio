#![allow(dead_code)]

use ffi::{RawFd, FD_SETSIZE};

use std::cmp;
use std::mem;
use std::cell::UnsafeCell;
use libc::{fd_set, FD_ZERO, FD_SET, FD_ISSET};

pub struct PosixFdSet {
    fds: UnsafeCell<fd_set>,
    max_fd: RawFd,
}

impl PosixFdSet {
    pub fn new() -> PosixFdSet {
        let fds = UnsafeCell::new(unsafe { mem::uninitialized() });
        unsafe { FD_ZERO(fds.get()); }
        PosixFdSet { fds: fds, max_fd: -1 }
    }

    pub fn reset(&mut self) {
        unsafe { FD_ZERO(self.fds.get()); }
        self.max_fd = -1;
    }

    pub fn set(&mut self, fd: RawFd) -> bool {
        if fd < FD_SETSIZE as _ {
            unsafe { FD_SET(fd, self.fds.get()) };
            self.max_fd = cmp::max(fd, self.max_fd);
            true
        } else {
            false
        }
    }

    pub fn is_set(&self, fd: RawFd) -> bool {
        unsafe { FD_ISSET(fd, self.fds.get()) }
    }

    pub fn max_fd(&self) -> RawFd {
        self.max_fd
    }
}
