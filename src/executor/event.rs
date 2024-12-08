use crate::error::OsError;
use std::task::{Waker, Context, Poll};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct Event {
    waker: Option<Waker>,
    read_op: Poll<Result<(), OsError>>,
    write_op: Poll<Result<(), OsError>>,
}

impl Event {
    pub fn new() -> Arc<Mutex<Event>> {
	Arc::new(Mutex::new(Event {
	    waker: None,
	    read_op: Poll::Pending,
	    write_op: Poll::Pending,
	}))
    }
	
    pub fn read_poll(&mut self, ctx: &mut Context, cnt: &AtomicUsize) -> Poll<Result<(), OsError>> {
	match self.read_op {
	    Poll::Pending => {
		self.waker = Some(ctx.waker().clone());
		cnt.fetch_add(1, Ordering::SeqCst);
		Poll::Pending
	    },
	    Poll::Ready(res) => {
		self.read_op = Poll::Pending;
		cnt.fetch_sub(1, Ordering::SeqCst);
		Poll::Ready(res)
	    },
	}
    }

    pub fn read_result(&mut self, res: Result<(), OsError>, vec: &mut Vec<Waker>) {
	if let Some(waker) = self.waker.take() {
	    self.read_op = Poll::Ready(res);
	    vec.push(waker);
	}
    }

    pub fn write_poll(&mut self, ctx: &mut Context, cnt: &AtomicUsize) -> Poll<Result<(), OsError>> {
	match self.write_op {
	    Poll::Pending => {
		self.waker = Some(ctx.waker().clone());
		cnt.fetch_add(1, Ordering::SeqCst);
		Poll::Pending
	    },
	    Poll::Ready(res) => {
		self.write_op = Poll::Pending;
		cnt.fetch_sub(1, Ordering::SeqCst);
		Poll::Ready(res)
	    },
	}
    }

    pub fn write_result(&mut self, res: Result<(), OsError>, vec: &mut Vec<Waker>) {
	if let Some(waker) = self.waker.take() {
	    self.write_op = Poll::Ready(res);
	    vec.push(waker);
	}
    }
}
