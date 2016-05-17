use super::{Expiry, ToExpiry};
use std::io;
use std::mem;
use std::cmp::{Eq, PartialEq, Ord, PartialOrd, Ordering};
use std::boxed::FnBox;
use std::sync::{Arc, Mutex};

pub type TimerHandler = Box<FnBox(io::Result<()>) + Send + 'static>;

struct TimerOp {
    expiry: Expiry,
    callback: TimerHandler,
}

impl Eq for TimerOp {
}

impl PartialEq for TimerOp {
    fn eq(&self, other: &Self) -> bool {
        let lhs: *const FnBox(io::Result<()>) = &*self.callback;
        let rhs: *const FnBox(io::Result<()>) = &*other.callback;
        self.expiry.eq(&other.expiry) && lhs.eq(&rhs)
    }
}

impl Ord for TimerOp {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.expiry.cmp(&other.expiry) {
            Ordering::Equal => {
                let lhs: *const FnBox(io::Result<()>) = &*self.callback;
                let rhs: *const FnBox(io::Result<()>) = &*other.callback;
                lhs.cmp(&rhs)
            },
            ord => ord,
        }
    }
}

impl PartialOrd for TimerOp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.expiry.partial_cmp(&other.expiry) {
            Some(Ordering::Equal) => {
                let lhs: *const FnBox(io::Result<()>) = &*self.callback;
                let rhs: *const FnBox(io::Result<()>) = &*other.callback;
                lhs.partial_cmp(&rhs)
            },
            ord => ord,
        }
    }
}

struct TimerEntry {
    ptr: *mut Option<TimerOp>,
}

unsafe impl Send for TimerEntry {
}

impl Eq for TimerEntry {
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        unsafe { &*self.ptr }.as_ref().unwrap().eq(unsafe { &*other.ptr }.as_ref().unwrap())
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe { &*self.ptr }.as_ref().unwrap().cmp(&unsafe { &*other.ptr }.as_ref().unwrap())
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unsafe { &*self.ptr }.as_ref().unwrap().partial_cmp(&unsafe { &*other.ptr }.as_ref().unwrap())
    }
}

struct TimerContainer {
    set: Vec<TimerEntry>,
}

impl TimerContainer {
    fn new() -> TimerContainer {
        TimerContainer {
            set: Vec::new(),
        }
    }

    fn insert(&mut self, ptr: *mut Option<TimerOp>) {
        assert!(unsafe { &*ptr }.is_some());

        let key = TimerEntry { ptr: ptr };
        let idx = match self.set.binary_search(&key) {
            Ok(len) => len + 1,
            Err(len) => len,
        };
        self.set.insert(idx, key);
    }

    fn remove(&mut self, ptr: *mut Option<TimerOp>) {
        assert!(unsafe { &*ptr }.is_some());

        let key = TimerEntry { ptr: ptr };
        let idx = self.set.binary_search(&key).unwrap();
        self.set.remove(idx);
    }

    fn len(&self) -> usize {
        self.set.len()
    }

    fn search(&self, expiry: &Expiry) -> usize {
        // TODO: binary search by expiry
        for (i, e) in self.set.iter().enumerate() {
            if &unsafe { &*e.ptr }.as_ref().unwrap().expiry > expiry {
                return i;
            }
        }
        self.len()
    }

    fn drain(&mut self, end: usize, vec: &mut Vec<TimerHandler>) {
        for e in self.set.drain(0..end) {
            let mut op = None;
            let ptr = unsafe { &mut *e.ptr };
            mem::swap(ptr, &mut op);
            vec.push(op.unwrap().callback)
        }
    }

    fn first_timeout(&self) -> Expiry {
        if let Some(e) = self.set.first() {
            unsafe { &*e.ptr }.as_ref().unwrap().expiry
        } else {
            Expiry::max_value()
        }
    }
}

impl Drop for TimerContainer {
    fn drop(&mut self) {
        assert!(self.len() == 0);
    }
}

#[derive(Clone)]
pub struct TimerQueue {
    queue: Arc<Mutex<TimerContainer>>,
}

impl TimerQueue {
    pub fn new() -> TimerQueue {
        TimerQueue {
            queue: Arc::new(Mutex::new(TimerContainer::new())),
        }
    }

    fn do_set_timer<E: ToExpiry>(&self, ev: &mut TimerEvent, expiry: E, callback: TimerHandler) -> Option<TimerHandler> {
        let mut op = TimerOp { expiry: expiry.to_expiry(), callback: callback, };
        let mut queue = self.queue.lock().unwrap();
        let ptr: *mut Option<TimerOp> = &mut *ev.timer_ptr;
        if let &mut Some(ref mut old_op) = ev.timer_ptr.as_mut() {
            queue.remove(ptr);
            mem::swap(old_op, &mut op);
            queue.insert(ptr);
            Some(op.callback)
        } else {
            unsafe { *ptr = Some(op) };
            queue.insert(ptr);
            None
        }
    }

    fn do_unset_timer(&self, ev: &mut TimerEvent) -> Option<TimerHandler> {
        if let Some(old_op) = {
            let mut opt = None;
            let mut queue = self.queue.lock().unwrap();
            let ptr: *mut Option<TimerOp> = &mut *ev.timer_ptr;
            if let &mut Some(_) = ev.timer_ptr.as_mut() {
                queue.remove(ptr);
            }
            mem::swap(&mut *ev.timer_ptr, &mut opt);
            opt
        } {
            Some(old_op.callback)
        } else {
            None
        }
    }

    pub fn drain_all(&self, vec: &mut Vec<TimerHandler>) {
        let mut queue = self.queue.lock().unwrap();
        let len = queue.len();
        queue.drain(len, vec);
    }

    pub fn drain_expired(&self, vec: &mut Vec<TimerHandler>) {
        let mut queue = self.queue.lock().unwrap();
        let len = queue.search(&Expiry::now());
        queue.drain(len, vec);
    }

    pub fn first_timeout(&self) -> Expiry {
        let queue = self.queue.lock().unwrap();
        queue.first_timeout()
    }
}

pub struct TimerEvent {
    timer_sv: TimerQueue,
    timer_ptr: Box<Option<TimerOp>>,
}

impl TimerEvent {
    pub fn new(sv: &TimerQueue) -> TimerEvent {
        TimerEvent {
            timer_sv: sv.clone(),
            timer_ptr: Box::new(None),
        }
    }

    pub fn set_timer<E: ToExpiry>(&mut self, expiry: E, callback: TimerHandler) -> Option<TimerHandler> {
        self.timer_sv.clone().do_set_timer(self, expiry, callback)
    }

    pub fn unset_timer(&mut self) -> Option<TimerHandler> {
        self.timer_sv.clone().do_unset_timer(self)
    }
}

impl Drop for TimerEvent {
    fn drop(&mut self) {
        let _ = self.unset_timer();
    }
}

#[test]
fn test_timer_set_unset() {
    use time;
    use std::thread;

    let sv = TimerQueue::new();
    let mut ev = TimerEvent::new(&sv);
    assert!(ev.unset_timer().is_none());
    assert!(ev.set_timer(time::SteadyTime::now(), Box::new(|_| {})).is_none());
    assert!(ev.set_timer(time::SteadyTime::now(), Box::new(|_| {})).is_some());
    thread::spawn(move || {
        assert!(ev.unset_timer().is_some());
        assert!(ev.unset_timer().is_none());
    }).join();
}
