use crate::error::OsError;
use std::collections::LinkedList;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Instant;
use std::{cmp, ptr};

#[derive(Debug)]
pub(crate) struct Event {
    waker: Option<Waker>,
    read_op: Poll<Result<(), OsError>>,
    write_op: Poll<Result<(), OsError>>,
}

impl Event {
    pub(crate) fn new() -> Arc<Mutex<Event>> {
        Arc::new(Mutex::new(Event {
            waker: None,
            read_op: Poll::Pending,
            write_op: Poll::Pending,
        }))
    }

    pub(crate) fn read_poll(
        &mut self,
        ctx: &mut Context,
        cnt: &AtomicUsize,
    ) -> Poll<Result<(), OsError>> {
        match self.read_op {
            Poll::Pending => {
                self.waker = Some(ctx.waker().clone());
                cnt.fetch_add(1, Ordering::SeqCst);
                Poll::Pending
            }
            Poll::Ready(res) => {
                self.read_op = Poll::Pending;
                cnt.fetch_sub(1, Ordering::SeqCst);
                Poll::Ready(res)
            }
        }
    }

    pub(crate) fn read_result(&mut self, res: Result<(), OsError>, vec: &mut Vec<Waker>) {
        if let Some(waker) = self.waker.take() {
            self.read_op = Poll::Ready(res);
            vec.push(waker);
        }
    }

    pub(crate) fn write_poll(
        &mut self,
        ctx: &mut Context,
        cnt: &AtomicUsize,
    ) -> Poll<Result<(), OsError>> {
        match self.write_op {
            Poll::Pending => {
                self.waker = Some(ctx.waker().clone());
                cnt.fetch_add(1, Ordering::SeqCst);
                Poll::Pending
            }
            Poll::Ready(res) => {
                self.write_op = Poll::Pending;
                cnt.fetch_sub(1, Ordering::SeqCst);
                Poll::Ready(res)
            }
        }
    }

    pub(crate) fn write_result(&mut self, res: Result<(), OsError>, vec: &mut Vec<Waker>) {
        if let Some(waker) = self.waker.take() {
            self.write_op = Poll::Ready(res);
            vec.push(waker);
        }
    }
}

struct DeadlineEvent {
    event: Arc<Mutex<Event>>,
    clock: Option<Instant>,
}

pub(crate) struct EventScheduler {
    list: LinkedList<DeadlineEvent>,
}

impl EventScheduler {
    pub(crate) fn new() -> Self {
        Self {
            list: LinkedList::new(),
        }
    }

    pub(crate) fn insert(&mut self, event: Arc<Mutex<Event>>) {
        self.list.push_back(DeadlineEvent {
            event: event,
            clock: None,
        })
    }

    pub(crate) fn remove(&mut self, event: &Arc<Mutex<Event>>) {
        let mut temp = LinkedList::new();
        while let Some(e) = self.list.pop_front() {
            if !ptr::addr_eq(&e.event, event) {
                temp.push_back(e)
            }
        }
        self.list.append(&mut temp);
    }

    pub(crate) fn update_deadline(&mut self, event: &Arc<Mutex<Event>>, time: Instant) -> bool {
        let mut nearest = time;
        for e in &mut self.list {
            if let Some(time) = e.clock {
                nearest = cmp::min(nearest, time)
            }
            if ptr::addr_eq(&e.event, event) {
                e.clock = Some(time)
            }
        }
        time == nearest
    }

    pub fn collect(&self) -> Vec<Arc<Mutex<Event>>> {
        self.list.iter().map(|e| e.event.clone()).collect()
    }

    pub(crate) fn timed_out(&self, now: Instant) -> Vec<Arc<Mutex<Event>>> {
        let mut vec = Vec::new();
        for e in &self.list {
            if let Some(time) = e.clock {
                if time < now {
                    vec.push(e.event.clone())
                }
            }
        }
        vec
    }
}
