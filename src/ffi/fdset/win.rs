use ffi::{fd_set, AsRawFd, RawFd, FD_SETSIZE, INVALID_SOCKET};

use std::mem;
use std::cmp;
use libc::{c_uint, free, malloc};
use ws2_32::__WSAFDIsSet;

pub struct WinFdSet {
    fds: *mut fd_set,
    capacity: usize,
    max_fd: RawFd,
}

impl Drop for WinFdSet {
    fn drop(&mut self) {
        unsafe { self.free() }
    }
}

impl WinFdSet {
    pub fn new() -> Self {
        WinFdSet {
            fds: unsafe { Self::alloc(FD_SETSIZE, 0) },
            capacity: FD_SETSIZE,
            max_fd: INVALID_SOCKET,
        }
    }

    unsafe fn alloc(capacity: usize, count: c_uint) -> *mut fd_set {
        let size = mem::size_of::<c_uint>() + capacity * mem::size_of::<RawFd>();
        let fds = malloc(size) as *mut fd_set;
        (*fds).fd_count = count;
        fds
    }

    unsafe fn free(&mut self) {
        free(self.fds as *mut _)
    }

    unsafe fn reserve(&mut self, len: usize) -> bool {
        if len <= self.capacity {
            return true;
        }

        let capacity = cmp::max(self.capacity + self.capacity / 2, len);
        let count = (*self.fds).fd_count;
        let fds = Self::alloc(capacity, count);
        if fds.is_null() {
            return false;
        }
        for i in 0..(count as isize) {
            let src = (*self.fds).fd_array.as_ptr().offset(i);
            let dst = (*fds).fd_array.as_mut_ptr().offset(i);
            *dst = *src;
        }
        self.free();
        self.fds = fds;
        self.capacity = capacity;
        true
    }

    pub fn as_raw(&mut self) -> *mut fd_set {
        self.fds
    }

    pub fn reset(&mut self) {
        unsafe {
            (*self.fds).fd_count = 0;
        }
        self.max_fd = INVALID_SOCKET;
    }

    pub fn set<T>(&mut self, t: &T) -> bool
    where
        T: AsRawFd,
    {
        let fd = t.as_raw_fd();
        unsafe {
            let len = (*self.fds).fd_count as usize;
            if !self.reserve(len + 1) {
                return false;
            }

            let fds = &mut *self.fds;
            *fds.fd_array.as_mut_ptr().offset(fds.fd_count as isize) = fd;
            fds.fd_count += 1;

            if self.max_fd == INVALID_SOCKET || self.max_fd < fd {
                self.max_fd = fd;
            }
        }
        true
    }

    pub fn is_set<T>(&self, t: &T) -> bool
    where
        T: AsRawFd,
    {
        unsafe { __WSAFDIsSet(t.as_raw_fd(), self.fds) != 0 }
    }

    pub fn max_fd(&self) -> RawFd {
        self.max_fd
    }
}
