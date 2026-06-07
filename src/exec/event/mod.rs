use crate::error::{OsError, Result};
use crate::socket::Timeout;
use std::cmp::PartialOrd;
use std::collections::LinkedList;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};
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

#[cfg(unix)]
use libc::c_int as NativeHandle;
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::SOCKET as NativeHandle;

#[derive(Clone)]
pub(super) struct Event {
    inner: Arc<(NativeHandle, Mutex<Inner>)>,
}

impl Event {
    pub(super) fn new(handle: NativeHandle) -> Self {
        let op = Mutex::new(Inner {
            readable_op: EventOp::Ready,
            writable_op: EventOp::Ready,
        });
        Self {
            inner: Arc::new((handle, op)),
        }
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

    #[allow(dead_code)]
    pub(super) fn from_raw_ptr(ev: *mut Event) -> Self {
        Self {
            inner: unsafe { Arc::from_raw(ev as *const (NativeHandle, Mutex<Inner>)) },
        }
    }

    #[allow(dead_code)]
    pub(super) fn as_raw_ptr(&self) -> *mut Event {
        let ev = Arc::into_raw(self.inner.clone());
        ev as *mut Event
    }

    #[allow(dead_code)]
    pub(super) unsafe fn as_native_handle(&self) -> NativeHandle {
        self.inner.0
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

    pub(super) fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    #[cfg(feature = "intr_timerfd")]
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
