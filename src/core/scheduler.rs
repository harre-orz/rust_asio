use super::{Deadline, Event};
use std::cmp::Ordering;
use std::collections::LinkedList;
use std::ptr;
use std::ptr::NonNull;
use std::sync::Mutex;
use std::task::Waker;

pub(super) struct Scheduler(Mutex<LinkedList<NonNull<Event>>>);

impl Scheduler {
    pub fn new() -> Self {
        Self(Mutex::new(LinkedList::new()))
    }

    pub fn pending_count(&self) -> usize {
        self.0.lock().unwrap().len()
    }

    pub fn add(&self, event: &Event) -> bool {
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
        unsafe { first.as_ref().eq(event) }
    }

    pub fn del(&self, event: &Event) {
        let mut tmp = LinkedList::new();
        let mut lst = self.0.lock().unwrap();
        while let Some(ev) = lst.pop_front() {
            match unsafe { ev.as_ref().cmp(event) } {
                Ordering::Equal => {}
                _ => tmp.push_back(event),
            }
        }
    }

    pub fn clear_overdue(&self, wakers: &mut Vec<Waker>, f: impl Fn(&Event, &mut Vec<Waker>)) {
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
            f(unsafe { ev.as_ref() }, wakers)
        }
    }

    pub fn clear_all(&self, wakers: &mut Vec<Waker>, f: impl Fn(&Event, &mut Vec<Waker>)) {
        let mut events = Vec::new();
        let mut lst = self.0.lock().unwrap();
        while let Some(ev) = lst.pop_front() {
            events.push(ev);
        }
        drop(lst);
        for ev in events {
            f(unsafe { ev.as_ref() }, wakers)
        }
    }
}

unsafe impl Send for Scheduler {}

unsafe impl Sync for Scheduler {}

#[test]
fn test_1() {
    use crate::primitive::Timeout;

    let ev1 = Event::new(());
    ev1.0.deadline.update(Timeout(2_000));
    let ev2 = Event::new(());
    ev2.0.deadline.update(Timeout(1_000));

    let scheduler = Scheduler::new();
    scheduler.add(&ev1.0);
    scheduler.add(&ev2.0);
}
