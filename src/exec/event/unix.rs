use crate::error::{OsError, Result};
use std::mem;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Debug)]
enum EventOp {
    Ready,
    Pending(Waker),
    Canceled,
    Neutral,
}

struct Inner {
    readable_op: EventOp,
    writable_op: EventOp,
}

impl Inner {
    fn read_poll(&mut self, ctx: &mut Context) -> Poll<Result<()>> {
        match self.readable_op {
            EventOp::Ready => {
                self.readable_op = EventOp::Neutral;
                Poll::Ready(Ok(()))
            }
            EventOp::Pending(_) => Poll::Pending,
            EventOp::Canceled => {
                self.readable_op = EventOp::Neutral;
                Poll::Ready(Err(OsError::OPERATION_CANCELED))
            }
            EventOp::Neutral => {
                self.readable_op = EventOp::Pending(ctx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn read_ready(&mut self, vec: &mut Vec<Waker>) {
        let mut event_op = EventOp::Ready;
        mem::swap(&mut event_op, &mut self.readable_op);
        if let EventOp::Pending(waker) = event_op {
            vec.push(waker);
        }
    }

    fn read_cancel(&mut self, vec: &mut Vec<Waker>) {
        let mut event_op = EventOp::Canceled;
        mem::swap(&mut event_op, &mut self.readable_op);
        if let EventOp::Pending(waker) = event_op {
            vec.push(waker);
        }
    }

    fn write_poll(&mut self, ctx: &mut Context) -> Poll<Result<()>> {
        match self.writable_op {
            EventOp::Neutral => {
                self.writable_op = EventOp::Pending(ctx.waker().clone());
                Poll::Pending
            }
            EventOp::Pending(_) => Poll::Pending,
            EventOp::Ready => {
                self.writable_op = EventOp::Neutral;
                Poll::Ready(Ok(()))
            }
            EventOp::Canceled => {
                self.writable_op = EventOp::Neutral;
                Poll::Ready(Err(OsError::OPERATION_CANCELED))
            }
        }
    }

    fn write_ready(&mut self, vec: &mut Vec<Waker>) {
        let mut event_op = EventOp::Ready;
        mem::swap(&mut event_op, &mut self.writable_op);
        if let EventOp::Pending(waker) = event_op {
            vec.push(waker);
        }
    }

    fn write_cancel(&mut self, vec: &mut Vec<Waker>) {
        let mut event_op = EventOp::Canceled;
        mem::swap(&mut event_op, &mut self.writable_op);
        if let EventOp::Pending(waker) = event_op {
            vec.push(waker);
        }
    }
}

#[derive(Clone)]
pub struct Event {
    inner: Arc<Mutex<Inner>>,
}

impl Event {
    pub(crate) fn new() -> Self {
        let op = Mutex::new(Inner {
            readable_op: EventOp::Ready,
            writable_op: EventOp::Ready,
        });
        Self {
            inner: Arc::new(op),
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

    pub(crate) fn ready(&self, readable: bool, writable: bool, vec: &mut Vec<Waker>) {
        let mut event = self.inner.lock().unwrap();
        if readable {
            event.read_ready(vec);
        }
        if writable {
            event.write_ready(vec);
        }
    }

    pub(crate) fn read_poll(&self, ctx: &mut Context) -> Poll<Result<()>> {
        let mut event = self.inner.lock().unwrap();
        event.read_poll(ctx)
    }

    pub(crate) fn write_poll(&self, ctx: &mut Context) -> Poll<Result<()>> {
        let mut event = self.inner.lock().unwrap();
        event.write_poll(ctx)
    }

    pub(crate) fn cancel(&self, vec: &mut Vec<Waker>) {
        let mut event = self.inner.lock().unwrap();
        event.read_cancel(vec);
        event.write_cancel(vec);
    }
}
