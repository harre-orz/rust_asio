use super::Event;
use crate::error::OsError;
use crate::socket::Socket;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;

pub(crate) struct Kqueue {}

impl Kqueue {
    pub(crate) fn new() -> Result<Self, OsError> {
        panic!()
    }

    pub(crate) fn register_socket(&self, soc: &Socket) -> Arc<Mutex<Event>> {
        panic!()
    }

    pub(crate) fn deregister_socket(&self, soc: &Socket, event: &Arc<Mutex<Event>>) {
        panic!()
    }

    pub(crate) fn stop_request(&self) {}

    pub(crate) fn update_schedule(&self, event: &Arc<Mutex<Event>>, time: Instant) {
        panic!()
    }

    pub(crate) fn ready_poll(&self) {
        panic!()
    }

    pub(crate) fn poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        Poll::Pending
    }
}
