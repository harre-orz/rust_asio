use crate::error::{OsError, Result};
use std::mem;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

enum EventOp {
    Neutral,
    Pending(Waker),
    Ok(usize),
    Err(OsError),
}

struct Inner {
    op: EventOp,
}

#[derive(Clone)]
pub struct Event {
    inner: Arc<Mutex<Inner>>,
}

impl Event {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                op: EventOp::Neutral,
            })),
        }
    }

    pub(crate) fn from_raw_ptr(ev: *mut Event) -> Self {
        Self {
            inner: unsafe { Arc::from_raw(ev as *const Mutex<Inner>) },
        }
    }

    pub(crate) fn as_raw_ptr(&self) -> *mut Event {
        let ev = Arc::into_raw(self.inner.clone());
        ev as *mut Event
    }

    pub(crate) fn poll(&self, ctx: &mut Context) -> Poll<Result<usize>> {
        let mut event = self.inner.lock().unwrap();
        match event.op {
            EventOp::Neutral => {
                event.op = EventOp::Pending(ctx.waker().clone());
                Poll::Pending
            }
            EventOp::Pending(_) => Poll::Pending,
            EventOp::Ok(len) => Poll::Ready(Ok(len)),
            EventOp::Err(err) => Poll::Ready(Err(err)),
        }
    }

    pub(crate) fn ready(&self, res: Result<usize>) {
        let waker = {
            let mut op = match res {
                Ok(len) => EventOp::Ok(len),
                Err(err) => EventOp::Err(err),
            };
            let mut event = self.inner.lock().unwrap();
            mem::swap(&mut op, &mut event.op);
            if let EventOp::Pending(waker) = op {
                waker
            } else {
                return;
            }
        };
        waker.wake();
    }

    pub(crate) fn cancel(&self, vec: &mut Vec<Waker>) {}
}
