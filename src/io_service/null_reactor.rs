use std::io;
use libc::close;
use super::{IoObject, IoService, ThreadInfo, RawFd, AsRawFd};

pub struct Reactor {
}

impl Reactor {
    pub fn new() -> Reactor {
        Reactor {
        }
    }

    pub fn poll(&self, block: bool, ti: &ThreadInfo) -> usize {
        0
    }

    pub fn cancel_all(&self, ti: &ThreadInfo) {
    }
}


pub struct IntrActor {
    fd: RawFd,
}

impl IntrActor {
    pub fn new(fd: RawFd) -> IntrActor {
        IntrActor {
            fd: fd,
        }
    }

    pub fn set_intr(&self, io: &IoService) {
    }

    pub fn unset_intr(&self, io: &IoService) {
    }
}

impl AsRawFd for IntrActor {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for IntrActor {
    fn drop(&mut self) {
        libc_ign!(close(self.fd));
    }
}


pub struct IoActor {
    io: IoService,
    fd: RawFd,
}

impl IoActor {
    pub fn new(io: &IoService, fd: RawFd) -> IoActor {
        IoActor {
            io: io.clone(),
            fd: fd,
        }
    }
}

impl Drop for IoActor {
    fn drop(&mut self) {
        libc_ign!(close(self.fd));
    }
}
