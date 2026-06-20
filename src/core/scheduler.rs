use super::{Deadline, Event};
use std::cmp::Ordering;
use std::collections::LinkedList;
use std::ptr;
use std::ptr::NonNull;
use std::sync::Mutex;

pub(super) struct Scheduler(Mutex<LinkedList<NonNull<Event>>>);

impl Scheduler {
    pub fn new() -> Self {
        Self(Mutex::new(LinkedList::new()))
    }

    pub fn pending_count(&self) -> usize {
        self.0.lock().unwrap().len()
    }

    pub fn add(&self, event: &Event) -> Option<Deadline> {
        let mut tmp = LinkedList::new();
        let mut lst = self.0.lock().unwrap();
        while let Some(ev) = lst.pop_front() {
            match unsafe { ev.as_ref().cmp(event) } {
                Ordering::Less => tmp.push_back(ev),
                _ => {}
            }
        }
        tmp.push_back(unsafe { NonNull::new_unchecked(ptr::from_ref(event).cast_mut()) });
        tmp.append(&mut *lst);
        lst.append(&mut tmp);
        let first = lst.pop_front().unwrap();
        if unsafe { first.as_ref().eq(event) } {
            Some(event.deadline.clone())
        } else {
            None
        }
    }

    pub fn del(&self, event: &Event) -> Option<Deadline> {
        let mut tmp = LinkedList::new();
        let mut lst = self.0.lock().unwrap();
        while let Some(ev) = lst.pop_front() {
            match unsafe { ev.as_ref().cmp(event) } {
                Ordering::Equal => {}
                _ => tmp.push_back(event),
            }
        }
        let first = lst.pop_front().unwrap();
        if unsafe { first.as_ref().eq(event) } {
            None
        } else {
            Some(unsafe { first.as_ref() }.deadline.clone())
        }
    }

    pub fn clear_overdue<F>(&self, mut f: F)
    where
        F: FnMut(&Event),
    {
        let now = Deadline::now();
        let mut events = Vec::new();
        let mut tmp = LinkedList::new();
        let mut lst = self.0.lock().unwrap();
        while let Some(ev) = lst.pop_front() {
            match unsafe { ev.as_ref() }.deadline.cmp(&now) {
                Ordering::Less => events.push(ev),
                _ => tmp.push_back(ev),
            }
        }
        lst.append(&mut tmp);
        drop(lst);
        for ev in events {
            f(unsafe { ev.as_ref() })
        }
    }

    pub fn clear_all<F>(&self, mut f: F)
    where
        F: FnMut(&Event),
    {
        let mut events = Vec::new();
        let mut lst = self.0.lock().unwrap();
        while let Some(ev) = lst.pop_front() {
            events.push(ev);
        }
        drop(lst);
        for ev in events {
            f(unsafe { ev.as_ref() })
        }
    }
}

unsafe impl Send for Scheduler {}

unsafe impl Sync for Scheduler {}
