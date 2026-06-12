use super::Event;
use std::collections::LinkedList;
use std::ptr;
use std::sync::Mutex;
use std::task::Waker;
use crate::primitive::Deadline;

struct DeadlineEvent {
    event: Event,
    timer: Deadline,
}

pub struct EventScheduler {
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

    pub(crate) fn insert_event(&self, event: &Event, timer: Deadline) -> bool {
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
            let mut ev = event.lock().unwrap();
            ev.cancel(vec);
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
            event.lock().unwrap().cancel(vec);
        }
    }
}
