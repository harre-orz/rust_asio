use crate::error::OsError;
use crate::executor::event::Event;
use std::os::fd::AsRawFd;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;

pub struct Kqueue {}

impl Kqueue {
    pub fn new() -> Result<Self, OsError> {
        panic!()
    }

    pub fn register_socket<F>(&self, soc: &F) -> Arc<Mutex<Event>>
    where
        F: AsRawFd,
    {
        panic!()
    }

    pub fn deregister_socket<F>(&self, soc: &F, event: &Arc<Mutex<Event>>)
    where
        F: AsRawFd,
    {
        panic!()
    }

    pub fn stop_request(&self) {}

    pub fn update_schedule(&self, event: &Arc<Mutex<Event>>, time: Instant) {
        panic!()
    }

    pub fn ready_poll(&self) {
        panic!()
    }

    pub fn poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        Poll::Pending
    }
}
