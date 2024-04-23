use crate::ffi;
use crate::error::OsError;
use std::io;
use std::os::fd::OwnedFd;
use std::time::Duration;

#[derive(Clone)]
pub struct IoContext {}

impl IoContext {
    pub fn new() -> Result<Self, OsError> {
        Ok(Self {})
    }

    pub fn is_stopped(&self) -> bool {
        true
    }

    pub fn run(&self) -> usize {
        0
    }

    pub(crate) async fn wait_for_readable(&self, soc: &OwnedFd, timeout: Duration) -> Result<(), OsError> {
        Ok(())
    }

    pub(crate) async fn wait_for_writable(&self, soc: &OwnedFd, timeout: Duration) -> Result<(), OsError> {
        Ok(())
    }
}
