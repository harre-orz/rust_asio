use std::io;
use std::mem;
use std::cmp::{Eq, PartialEq, Ord, PartialOrd, Ordering};
use std::cell::UnsafeCell;
use std::boxed::FnBox;
use std::sync::{Arc, Mutex};
use super::{UseService, Handler, Expiry};

struct TimerOp {
    expiry: Expiry,
    callback: Handler,
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

struct TimerObject {
    timer_op: Option<TimerOp>,
}

struct TimerEntry {
    ptr: *mut TimerObject,
}

unsafe impl Send for TimerEntry {}

impl Eq for TimerEntry {
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        unsafe { &*self.ptr }.timer_op.as_ref().unwrap().eq(unsafe { &*other.ptr }.timer_op.as_ref().unwrap())
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe { &*self.ptr }.timer_op.as_ref().unwrap().cmp(&unsafe { &*other.ptr }.timer_op.as_ref().unwrap())
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unsafe { &*self.ptr }.timer_op.as_ref().unwrap().partial_cmp(&unsafe { &*other.ptr }.timer_op.as_ref().unwrap())
    }
}

pub struct TimerQueue {
    mutex: Mutex<Vec<TimerEntry>>,
}

impl TimerQueue {
    pub fn new() -> TimerQueue {
        TimerQueue {
            mutex: Mutex::new(Vec::new()),
        }
    }

    fn find_timeout(queue: &Vec<TimerEntry>, expiry: &Expiry) -> usize {
        // TODO: binary search by expiry
        for (i, e) in queue.iter().enumerate() {
            if &unsafe { &*e.ptr }.timer_op.as_ref().unwrap().expiry > expiry {
                return i;
            }
        }
        queue.len()
    }

    fn insert(queue: &mut Vec<TimerEntry>, ptr: *mut TimerObject) {
        assert!(unsafe { &*ptr }.timer_op.is_some());

        let key = TimerEntry { ptr: ptr };
        let idx = match queue.binary_search(&key) {
            Ok(len) => len + 1,
            Err(len) => len,
        };
        queue.insert(idx, key);
    }

    fn remove(queue: &mut Vec<TimerEntry>, ptr: *mut TimerObject) {
        assert!(unsafe { &*ptr }.timer_op.is_some());

        let key = TimerEntry { ptr: ptr };
        let idx = queue.binary_search(&key).unwrap();
        queue.remove(idx);
    }

    fn do_set_timer(&self, ptr: *mut TimerObject, expiry: Expiry, callback: Handler) -> Option<Handler> {
        let mut timer_op = TimerOp { expiry: expiry, callback: callback };
        let mut queue = self.mutex.lock().unwrap();
        if let Some(old_op) = unsafe { &mut *ptr }.timer_op.as_mut() {
            Self::remove(&mut queue, ptr);
            mem::swap(old_op, &mut timer_op);
            Self::insert(&mut queue, ptr);
            Some(timer_op.callback)
        } else {
            unsafe { &mut *ptr }.timer_op = Some(timer_op);
            Self::insert(&mut queue, ptr);
            None
        }
    }

    fn do_unset_timer(&self, ptr: *mut TimerObject) -> Option<Handler> {
        if let Some(old_op) = {
            let mut timer_op = None;
            let mut queue = self.mutex.lock().unwrap();
            if let Some(_) = unsafe { &mut *ptr }.timer_op.as_mut() {
                Self::remove(&mut queue, ptr);
            }
            mem::swap(&mut unsafe { &mut *ptr }.timer_op, &mut timer_op);
            timer_op
        } {
            Some(old_op.callback)
        } else {
            None
        }
    }

    pub fn drain_all(&self, vec: &mut Vec<Handler>) -> usize {
        let mut queue = self.mutex.lock().unwrap();
        let len = queue.len();
        Self::drain(&mut queue, len, vec);
        0
    }

    pub fn drain_expired(&self, vec: &mut Vec<Handler>) -> usize {
        let mut queue = self.mutex.lock().unwrap();
        let len = Self::find_timeout(&queue, &Expiry::now());
        Self::drain(&mut queue, len, vec);
        queue.len()
    }

    fn drain(queue: &mut Vec<TimerEntry>, len: usize, vec: &mut Vec<Handler>) {
        for e in queue.drain(..len) {
            let mut timer_op = None;
            mem::swap(&mut unsafe { &mut *e.ptr }.timer_op, &mut timer_op);
            vec.push(timer_op.unwrap().callback)
        }
    }

    pub fn first_timeout(&self) -> Expiry {
        let queue = self.mutex.lock().unwrap();
        if let Some(e) = queue.first() {
            unsafe { &*e.ptr }.timer_op.as_ref().unwrap().expiry
        } else {
            Expiry::max_value()
        }
    }
}

pub struct TimerActor {
    timer_ptr: Box<UnsafeCell<TimerObject>>,
}

impl TimerActor {
    pub fn new() -> TimerActor {
        TimerActor {
            timer_ptr: Box::new(UnsafeCell::new(TimerObject {
                timer_op: None,
            })),
        }
    }

    pub fn set_timer<T: UseService<TimerQueue>>(&self, io: &T, expiry: Expiry, callback: Handler) -> Option<Handler> {
        io.use_service().do_set_timer(unsafe { &mut *self.timer_ptr.get() }, expiry, callback)
    }

    pub fn unset_timer<T: UseService<TimerQueue>>(&self, io: &T) -> Option<Handler> {
        io.use_service().do_unset_timer(unsafe { &mut *self.timer_ptr.get() })
    }
}

#[test]
fn test_timer_set_unset() {
    use std::thread;
    use time;
    use IoService;
    use super::ToExpiry;

    let io = IoService::new();
    let ev = TimerActor::new();
    assert!(ev.unset_timer(&io).is_none());
    assert!(ev.set_timer(&io, time::SteadyTime::now().to_expiry(), Box::new(|_| {})).is_none());
    assert!(ev.set_timer(&io, time::SteadyTime::now().to_expiry(), Box::new(|_| {})).is_some());
    thread::spawn(move || {
        assert!(ev.unset_timer(&io).is_some());
        assert!(ev.unset_timer(&io).is_none());
    }).join();
}
