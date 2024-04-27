use crate::error::OsError;
use crate::ffi::ConnectedSocket;
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

    pub async fn wait_for_readable(
        &self,
        soc: &ConnectedSocket,
        timeout: Duration,
    ) -> Result<(), OsError> {
        Ok(())
    }

    pub async fn wait_for_writable(
        &self,
        soc: &ConnectedSocket,
        timeout: Duration,
    ) -> Result<(), OsError> {
        Ok(())
    }

    pub fn register_socket(&self, soc: &ConnectedSocket) {}

    pub fn deregister_socket(&self, soc: &ConnectedSocket) {}
}
