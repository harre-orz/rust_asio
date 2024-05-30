use crate::error::OsError;
use crate::ffi::Timeout;
use crate::ConnectedSocket;
use libc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

mod ffi {}

pub struct Inner {}

#[derive(Clone)]
pub(super) struct SelectEvent(Arc<Mutex<Inner>>);

impl SelectEvent {
    pub fn read_reset(self, select: &Select, timeout: Timeout) {}

    pub fn write_reset(self, select: &Select, timeout: Timeout) {}

    pub fn read_poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        Poll::Pending
    }

    pub fn write_poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        Poll::Pending
    }
}

pub(super) struct Select {
    read_fds: DeadlineEvent,
    write_fds: DeadlineEvent,
}

impl Select {
    pub fn new() -> Result<Self, OsError> {
        Ok(Self {})
    }

    pub fn register_socket(&self, soc: &ConnectedSocket) -> SelectEvent {
        SelectEvent(Arc::new(Mutex::new(Inner {})))
    }

    pub fn deregister_socket(&self, soc: &ConnectedSocket) {}

    pub fn stop(&self) {}

    pub fn poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        ffi::select();
        Poll::Pending
    }
}
