use crate::OsError;
use crate::ffi;
use std::os::fd::OwnedFd;
use std::time::Duration;
use std::io;

#[derive(Clone)]
pub struct IoContext {
}

impl IoContext {
    pub fn new() -> Result<Self, OsError> {
        Ok(Self { })
    }

    pub fn is_stopped(&self) -> bool {
        true
    }

    pub fn run(&self) -> usize {
        0
    }
}


pub struct YieldContext {
    ctx: IoContext,
}

impl YieldContext {
    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub(crate) fn wait_for_connected(&mut self, soc: &OwnedFd, wait: Duration) -> Result<(), OsError> {
        ffi::wait_writable(soc, wait)
    }

    pub(crate) fn wait_for_readable(&mut self, soc: &OwnedFd, wait: Duration) -> Result<(), OsError> {
        ffi::wait_readable(soc, wait)
    }

    pub(crate) fn wait_for_writable(&mut self, soc: &OwnedFd, wait: Duration) -> Result<(), OsError> {
        ffi::wait_writable(soc, wait)
    }
}


impl IoContext {
    pub fn spawn<F>(&self, f: F) -> Result<(), io::Error>
    where
        F: FnOnce(&mut YieldContext)
    {
        let mut yield_ctx = YieldContext { ctx: self.clone() };
        f(&mut yield_ctx);
        Ok(())
    }
}
