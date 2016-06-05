use std::io;
use std::mem;
use std::cmp::{Eq, PartialEq, Ord, PartialOrd, Ordering};
use std::cell::UnsafeCell;
use std::boxed::FnBox;
use std::sync::Mutex;
use {IoObject, IoService};
use super::{UseService, Handler, Expiry};

struct TimerOp {
    expiry: Expiry,
    callback: Handler,
    id: usize,
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

    fn do_set_timer(&self, ptr: *mut TimerObject, expiry: Expiry, callback: Handler, id: usize) -> Option<Handler> {
        let mut timer_op = TimerOp { expiry: expiry, callback: callback, id: id };
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

    pub fn drain_all(&self, vec: &mut Vec<(usize, Handler)>) -> usize {
        let mut queue = self.mutex.lock().unwrap();
        let len = queue.len();
        Self::drain(&mut queue, len, vec);
        0
    }

    pub fn drain_expired(&self, vec: &mut Vec<(usize, Handler)>) -> usize {
        let mut queue = self.mutex.lock().unwrap();
        let len = Self::find_timeout(&queue, &Expiry::now());
        Self::drain(&mut queue, len, vec);
        queue.len()
    }

    fn drain(queue: &mut Vec<TimerEntry>, len: usize, vec: &mut Vec<(usize, Handler)>) {
        for e in queue.drain(..len) {
            let mut timer_op = None;
            mem::swap(&mut unsafe { &mut *e.ptr }.timer_op, &mut timer_op);
            let op = timer_op.unwrap();
            vec.push((op.id, op.callback))
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
    io: IoService,
    timer_ptr: Box<UnsafeCell<TimerObject>>,
}

impl TimerActor {
    pub fn register(io: &IoService) -> TimerActor {
        TimerActor {
            io: io.clone(),
            timer_ptr: Box::new(UnsafeCell::new(TimerObject {
                timer_op: None,
            })),
        }
    }

    pub fn unregister(&self) {
    }

    fn use_service(&self) -> &TimerQueue {
        self.io.use_service()
    }

    pub fn set_timer(&self, expiry: Expiry, callback: Handler, _: usize) -> Option<Handler> {
        let res = self.use_service().do_set_timer(unsafe { &mut *self.timer_ptr.get() }, expiry, callback, 0);
        self.io.timeout(self.use_service().first_timeout());
        res
    }

    pub fn unset_timer(&self) -> Option<Handler> {
        let res = self.use_service().do_unset_timer(unsafe { &mut *self.timer_ptr.get() });
        self.io.timeout(self.use_service().first_timeout());
        res
    }
}

impl IoObject for TimerActor {
    fn io_service(&self) -> IoService {
        self.io.clone()
    }
}

#[test]
fn test_timer_set_unset() {
    use std::thread;
    use time;
    use super::ToExpiry;

    let io = IoService::new();
    let ev = TimerActor::register(&io);
    assert!(ev.unset_timer().is_none());
    assert!(ev.set_timer(time::SteadyTime::now().to_expiry(), Box::new(|_| {}), 0).is_none());
    assert!(ev.set_timer(time::SteadyTime::now().to_expiry(), Box::new(|_| {}), 0).is_some());
    thread::spawn(move || {
        assert!(ev.unset_timer().is_some());
        assert!(ev.unset_timer().is_none());
        ev.unregister();
    }).join();
}

#[test]
fn test_ordered_queue() {
    use time;
    use super::ToExpiry;

    let io = IoService::new();
    let sv: &TimerQueue = io.use_service();
    let ev1 = TimerActor::register(&io);
    let ev2 = TimerActor::register(&io);
    let ev3 = TimerActor::register(&io);
    let now = time::SteadyTime::now();
    ev1.set_timer((now + time::Duration::minutes(1)).to_expiry(), Box::new(|_| {}), 0);
    ev2.set_timer(now.to_expiry(), Box::new(|_| {}), 0);
    assert!(sv.first_timeout() == now.to_expiry());
    ev3.set_timer((now - time::Duration::seconds(1)).to_expiry(), Box::new(|_| {}), 0);
    assert!(sv.first_timeout() == (now - time::Duration::seconds(1)).to_expiry());
    let _ = ev2.unset_timer();
    let mut vec = Vec::new();
    sv.drain_expired(&mut vec);
    assert!(vec.len() == 1);
    assert!(sv.first_timeout() == (now + time::Duration::minutes(1)).to_expiry());
    let _ = ev1.unset_timer();
    ev1.unregister();
    ev2.unregister();
    ev3.unregister();
}
