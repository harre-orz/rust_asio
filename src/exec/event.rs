use crate::error::{OsError, Result};
use crate::socket::{Fd, Timeout};
use std::cmp::PartialOrd;
use std::collections::LinkedList;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Instant;
use std::{mem, ptr};

#[derive(Debug)]
enum EventOp {
    Wait,
    Pending(Waker),
    Ready,
    Canceled,
}

struct Inner {
    readable_op: EventOp,
    writable_op: EventOp,
}

impl Inner {
    fn read_poll(&mut self, ctx: &mut Context) -> Poll<Result<()>> {
        match self.readable_op {
            EventOp::Wait => {
                self.readable_op = EventOp::Pending(ctx.waker().clone());
                Poll::Pending
            }
            EventOp::Pending(_) => Poll::Pending,
            EventOp::Ready => {
                self.readable_op = EventOp::Wait;
                Poll::Ready(Ok(()))
            }
            EventOp::Canceled => {
                self.readable_op = EventOp::Wait;
                Poll::Ready(Err(OsError::OPERATION_CANCELED))
            }
        }
    }

    fn read_ok(&mut self, vec: &mut Vec<Waker>) {
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
            EventOp::Wait => {
                self.writable_op = EventOp::Pending(ctx.waker().clone());
                Poll::Pending
            }
            EventOp::Pending(_) => Poll::Pending,
            EventOp::Ready => {
                self.writable_op = EventOp::Wait;
                Poll::Ready(Ok(()))
            }
            EventOp::Canceled => {
                self.writable_op = EventOp::Wait;
                Poll::Ready(Err(OsError::OPERATION_CANCELED))
            }
        }
    }

    fn write_ok(&mut self, vec: &mut Vec<Waker>) {
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
pub(super) struct Event {
    inner: Arc<(libc::c_int, Mutex<Inner>)>,
}

impl Event {
    pub(super) fn new(fd: &Fd) -> Self {
        let op = Mutex::new(Inner {
            readable_op: EventOp::Ready,
            writable_op: EventOp::Ready,
        });
        Self {
            inner: Arc::new((unsafe { fd.as_raw_fd() }, op)),
        }
    }

    pub(super) unsafe fn from_raw_ptr(ptr: *mut libc::c_void) -> Self {
        let inner = unsafe { Arc::from_raw(ptr.cast()) };
        Self { inner: inner }
    }

    pub(super) fn as_raw_ptr(&self) -> *mut libc::c_void {
        Arc::into_raw(self.clone().inner) as *mut libc::c_void
    }

    #[cfg(feature = "poll_select")]
    pub(super) unsafe fn as_raw_fd(&self) -> libc::c_int {
        self.inner.0
    }

    pub(super) fn ready(&self, readable: bool, writable: bool, vec: &mut Vec<Waker>) {
        let mut event = self.inner.1.lock().unwrap();
        if readable {
            event.read_ok(vec);
        }
        if writable {
            event.write_ok(vec);
        }
    }

    pub(super) fn read_poll(&self, ctx: &mut Context) -> Poll<Result<()>> {
        let mut event = self.inner.1.lock().unwrap();
        event.read_poll(ctx)
    }

    pub(super) fn write_poll(&self, ctx: &mut Context) -> Poll<Result<()>> {
        let mut event = self.inner.1.lock().unwrap();
        event.write_poll(ctx)
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub(super) struct Deadline(Instant);

impl Deadline {
    pub(super) fn new(timeout: Timeout) -> Self {
        Self(Instant::now() + timeout.into_duration())
    }

    pub(super) fn now() -> Self {
        Deadline(Instant::now())
    }

    #[allow(dead_code)]
    pub(super) fn as_relative_timespec(&self) -> libc::timespec {
        let duration = self.0.elapsed();
        libc::timespec {
            tv_sec: duration.as_secs() as libc::time_t,
            tv_nsec: duration.subsec_nanos() as libc::c_long,
        }
    }

    #[allow(dead_code)]
    pub(super) fn as_relative_timeval(&self) -> libc::timeval {
        let duration = self.0.elapsed();
        libc::timeval {
            tv_sec: duration.as_secs() as libc::time_t,
            #[cfg(target_os = "macos")]
            tv_usec: duration.subsec_micros() as libc::suseconds_t,
            #[cfg(not(target_os = "macos"))]
            tv_usec: duration.subsec_micros() as libc::c_long,
        }
    }

    #[allow(dead_code)]
    pub(super) fn as_relative_millis(&self) -> i32 {
        let duration = self.0.elapsed();
        duration.as_millis() as i32
    }

    #[allow(dead_code)]
    pub(super) fn as_absolute_timespec(&self) -> libc::timespec {
        unimplemented!("")
    }
}

struct DeadlineEvent {
    event: Event,
    timer: Deadline,
}

pub(super) struct EventScheduler {
    list: Mutex<LinkedList<DeadlineEvent>>,
}

impl EventScheduler {
    pub(super) fn new() -> Self {
        Self {
            list: Mutex::new(LinkedList::new()),
        }
    }

    pub(super) fn pending_count(&self) -> usize {
        self.list.lock().unwrap().len()
    }

    pub(super) fn insert_event(&self, event: &Event, timer: Deadline) -> bool {
        let mut list = self.list.lock().unwrap();
        list.push_back(DeadlineEvent {
            event: event.clone(),
            timer: timer,
        });
        ptr::addr_eq(&list.front().unwrap().event, event)
    }

    pub(super) fn update_event(&self, event: &Event, now: Deadline, vec: &mut Vec<Waker>) {
        let mut target_event = None;
        {
            let mut list_mut = LinkedList::new();
            let mut list = self.list.lock().unwrap();
            while let Some(ev) = list.pop_front() {
                if ptr::addr_eq(&ev.event, event) {
                    if ev.timer > now {
                        target_event = Some(ev.event);
                    }
                } else {
                    list_mut.push_back(ev)
                }
            }
            list.append(&mut list_mut);
        }
        if let Some(event) = target_event {
            let mut event = event.inner.1.lock().unwrap();
            event.read_cancel(vec);
            event.write_cancel(vec);
        }
    }

    pub(super) fn cancel_all_events(&self, vec: &mut Vec<Waker>) {
        let mut events = Vec::new();
        {
            let mut list = self.list.lock().unwrap();
            while let Some(ev) = list.pop_front() {
                events.push(ev.event);
            }
        }
        for event in events {
            let mut event = event.inner.1.lock().unwrap();
            event.read_cancel(vec);
            event.write_cancel(vec);
        }
    }
}
