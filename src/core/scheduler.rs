use super::AsyncEvent;
use crate::primitive::Deadline;
use std::collections::LinkedList;
use std::ptr;
use std::sync::Mutex;
use std::task::Waker;

struct DeadlineEvent {
    event: AsyncEvent<()>,
    timer: Deadline,
}

pub(super) struct Scheduler {
    list: Mutex<LinkedList<DeadlineEvent>>,
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

    pub fn insert_event<T>(&self, event: &AsyncEvent<()>, timer: Deadline) -> bool {
        let mut list = self.list.lock().unwrap();
        list.push_back(DeadlineEvent {
            event: event.clone(),
            timer: timer,
        });
        ptr::addr_eq(&list.front().unwrap().event, event)
    }

    pub fn update_event<T>(&self, event: &AsyncEvent<T>, now: Deadline, vec: &mut Vec<Waker>) {
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
            event.cancel(vec);
        }
    }

    pub fn cancel_all_events(&self, vec: &mut Vec<Waker>) {
        let mut events = Vec::new();
        {
            let mut list = self.list.lock().unwrap();
            while let Some(ev) = list.pop_front() {
                events.push(ev.event);
            }
        }
        for event in events {
            event.cancel(vec);
        }
    }
}
