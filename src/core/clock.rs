use super::AsyncEvent;
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

pub(super) struct Scheduler(Mutex<LinkedList<(Deadline, *const ())>>);

impl Scheduler {
    pub fn new() -> Self {
        Self(Mutex::new(LinkedList::new()))
    }

    pub fn pending_count(&self) -> usize {
        self.0.lock().unwrap().len()
    }

    pub fn add_event(&self, deadline: Deadline, ptr: *const ()) -> bool {
        false
    }

    pub fn del_events<T, F>(&self, vec: &mut Vec<Waker>, ev: &[T], f: F)
    where
        F: Fn(&T) -> *const (),
    {
    }

    pub fn cancel_all_events<F>(&self, mut cancel: F)
        where
        F: FnMut(*const ()),
    {
        let mut events = Vec::new();
        {
            let mut list = self.0.lock().unwrap();
            while let Some(ev) = list.pop_front() {
                events.push(ev);
            }
        }
        for event in events {
            cancel(event.1);
        }
    }
}
