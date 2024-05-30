use super::Event;
use crate::ffi::Monotonic;
use std::cmp;
use std::collections::{BTreeSet, HashSet};
use std::time::Duration;

#[derive(Eq, PartialEq)]
pub(super) struct DeadlineEvent(pub Event, Monotonic);

impl cmp::Ord for DeadlineEvent {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.1.cmp(&other.1).reverse() {
            cmp::Ordering::Equal => self.0.cmp(&other.0),
            cmp => cmp,
        }
    }
}

impl cmp::PartialOrd for DeadlineEvent {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

pub(super) struct DeadlineEventSet {
    tree: BTreeSet<DeadlineEvent>,
    last: Monotonic,
}

impl DeadlineEventSet {
    pub fn new() -> Self {
        Self {
            tree: BTreeSet::new(),
            last: Monotonic::now(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    pub fn insert_event(&mut self, event: Event, deadline: Monotonic) -> Option<Monotonic> {
        self.tree.insert(DeadlineEvent(event, deadline));
        let last = self.tree.last().unwrap().1;
        if self.last != last {
            self.last = last;
            Some(last)
        } else {
            None
        }
    }

    pub fn remove_events(&mut self, events: &HashSet<Event>) {
        let mut temp = BTreeSet::new();
        while let Some(event) = self.tree.pop_first() {
            if let Some(_) = events.get(&event.0) {
                temp.insert(event);
            }
        }
        self.tree.append(&mut temp);
    }

    pub fn update_timeout(&mut self) -> Option<Monotonic> {
        let last = if let Some(event) = self.tree.last() {
            event.1
        } else {
            Monotonic::now() + Duration::new(1_000_000, 0)
        };
        if self.last != last {
            self.last = last;
            Some(last)
        } else {
            None
        }
    }

    pub fn detach_timeout_events(
        &mut self,
        event: Event,
        deadline: Monotonic,
    ) -> BTreeSet<DeadlineEvent> {
        self.tree.split_off(&DeadlineEvent(event, deadline))
    }

    pub fn into_hashset(&mut self) -> HashSet<Event> {
        let mut events = HashSet::new();
        while let Some(event) = self.tree.pop_first() {
            events.insert(event.0);
        }
        events
    }
}

// #[test]
// fn test_ordering() {
//     use std::time::Duration;
//
//     let now = Monotonic::now();
//     let mut data: BTreeSet<DeadlineEpollEvent> = BTreeSet::new();
//
//     let ev1 = EpollEvent::socket();
//     data.insert(DeadlineEpollEvent(now + Duration::new(10, 0), ev1.clone())); // 1
//
//     let ev2 = EpollEvent::socket();
//     data.insert(DeadlineEpollEvent(now + Duration::new(30, 0), ev2.clone())); // 3
//
//     let ev3 = EpollEvent::socket();
//     data.insert(DeadlineEpollEvent(now + Duration::new(50, 0), ev3.clone()));
//
//     let ev4 = EpollEvent::socket();
//     data.insert(DeadlineEpollEvent(now + Duration::new(40, 0), ev4.clone()));
//
//     let ev5 = EpollEvent::socket();
//     data.insert(DeadlineEpollEvent(now + Duration::new(20, 0), ev5.clone())); // 2
//
//     let now = now + Duration::new(35, 0);
//     let dummy = EpollEvent::socket();
//     let mut dead = data.split_off(&DeadlineEpollEvent(now, dummy));
//     if let Some(DeadlineEpollEvent(_, ev)) = dead.pop_last() {
//         assert_eq!(ev, ev1);
//     }
//     if let Some(DeadlineEpollEvent(_, ev)) = dead.pop_last() {
//         assert_eq!(ev, ev5);
//     }
//     if let Some(DeadlineEpollEvent(_, ev)) = dead.pop_last() {
//         assert_eq!(ev, ev2);
//     }
//     assert_eq!(dead.is_empty(), true);
// }
