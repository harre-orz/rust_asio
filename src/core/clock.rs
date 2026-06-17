use super::AsyncEvent;
use crate::primitive::Timeout;
use std::collections::LinkedList;
use std::ptr;
use std::sync::Mutex;
use std::task::Waker;

#[cfg(any(windows, feature = "eventfd", feature = "pipe"))]
mod instant;
#[cfg(any(windows, feature = "eventfd", feature = "pipe"))]
pub(super) use self::instant::Deadline;

#[cfg(not(any(windows, feature = "eventfd", feature = "pipe")))]
mod timespec;
#[cfg(not(any(windows, feature = "eventfd", feature = "pipe")))]
pub(super) use self::timespec::Deadline;

pub(super) struct Scheduler {
    list: Mutex<LinkedList<(Deadline, *const ())>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            list: Mutex::new(LinkedList::new()),
        }
    }

    pub fn pending_count(&self) -> usize {
        self.list.lock().unwrap().len()
    }

    pub fn add_event(&self, ev: *const (), timer: Timeout) -> Option<Deadline> {
        None
    }

    pub fn del_event(&self, ev: *const ()) -> Option<Deadline> {
        None
    }

    pub fn clear_overdue<F>(&self, mut f: F)
    where
        F: FnMut(*const ()),
    {
    }

    pub fn cancel_all_events<F>(&self, mut f: F)
    where
        F: FnMut(*const ()),
    {
        let mut events = Vec::new();
        {
            let mut list = self.list.lock().unwrap();
            while let Some(ev) = list.pop_front() {
                events.push(ev);
            }
        }
        for event in events {
            f(event.1)
        }
    }
}

unsafe impl Send for Scheduler {}

unsafe impl Sync for Scheduler {}
