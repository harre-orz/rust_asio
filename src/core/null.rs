use ffi::{AsRawFd, RawFd, SystemError, close};
use core::{Expiry, InnerTimerPtr, IoContext, Perform, ThreadIoContext};

use std::sync::Mutex;

pub struct NullFd(RawFd);

impl NullFd {
    pub fn socket(fd: RawFd) -> Self {
        NullFd(fd)
    }
}

impl NullFd {
    pub fn add_read_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        op.perform(this, err)
    }

    pub fn add_write_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        op.perform(this, err)
    }

    pub fn next_read_op(&mut self, _: &mut ThreadIoContext) {}

    pub fn next_write_op(&mut self, _: &mut ThreadIoContext) {}

    pub fn cancel_ops(&mut self, _: &IoContext) {}
}

impl Drop for NullFd {
    fn drop(&mut self) {
        close(self.0)
    }
}

impl AsRawFd for NullFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

pub struct NullReactor {
    pub tq: Mutex<Vec<InnerTimerPtr>>,
}

unsafe impl Send for NullReactor {}

unsafe impl Sync for NullReactor {}

impl NullReactor {
    pub fn new() -> Result<Self, SystemError> {
        Ok(NullReactor {
            tq: Mutex::default(),
        })
    }

    pub fn poll(&self, _: bool, _: &mut ThreadIoContext) {}

    pub fn register_socket(&self, _: &NullFd) {}

    pub fn deregister_socket(&self, _: &NullFd) {}

    pub fn interrupt(&self) {}

    pub fn reset_timeout(&self, _: Expiry) {}
}
