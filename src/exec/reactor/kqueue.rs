use crate::error::OsError;
use crate::exec::event::Event;
use crate::ffi::socket::Socket;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;

pub struct Kqueue {}

impl Kqueue {
    pub fn new() -> Result<Self, OsError> {
        panic!()
    }

    pub fn register_socket(&self, soc: &Socket) -> Arc<Mutex<Event>> {
        panic!()
    }

    pub fn deregister_socket(&self, soc: &Socket, event: &Arc<Mutex<Event>>) {
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
