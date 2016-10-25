use std::mem;
use std::cmp::Ordering;
use std::sync::Mutex;
use unsafe_cell::{UnsafeBoxedCell};
use error::{ECANCELED};
use clock::Expiry;
use super::{IoObject, IoService, Callback, ThreadInfo};

struct Op {
    expiry: Expiry,
    callback: Callback,
}

struct Entry {
    op: Option<Op>,
}

struct EntryPtr(*mut Entry);

impl Eq for EntryPtr {
}

impl PartialEq for EntryPtr {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Ord for EntryPtr {
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs = &unsafe { &*self.0 }.op.as_ref().unwrap().expiry;
        let rhs = &unsafe { &*other.0 }.op.as_ref().unwrap().expiry;

        match lhs.cmp(rhs) {
            Ordering::Equal => self.0.cmp(&other.0),
            cmp => cmp,
        }
    }
}

impl PartialOrd for EntryPtr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

unsafe impl Send for EntryPtr {
}

unsafe impl Sync for EntryPtr {
}

fn insert(queue: &mut Vec<EntryPtr>, ptr: *mut Entry) -> bool {
    debug_assert!(unsafe { &*ptr }.op.is_some());

    let key = EntryPtr(ptr);
    let idx = match queue.binary_search(&key) {
        Ok(len) => len + 1,
        Err(len) => len,
    };
    queue.insert(idx, key);
    idx == 0
}

fn remove(queue: &mut Vec<EntryPtr>, ptr: *mut Entry) -> bool {
    debug_assert!(unsafe { &*ptr }.op.is_some());

    let key = EntryPtr(ptr);
    let idx = queue.binary_search(&key).unwrap();
    queue.remove(idx);
    idx == 0
}

fn find_timeout(queue: &Vec<EntryPtr>, expiry: Expiry) -> usize {
    for (i, ptr) in queue.iter().enumerate() {
        if unsafe { &*ptr.0 }.op.as_ref().unwrap().expiry > expiry {
            return i;
        }
    }
    queue.len()
}

fn drain(queue: &mut Vec<EntryPtr>, len: usize, ti: &ThreadInfo) {
    for ptr in queue.drain(..len) {
        let Op { expiry:_, callback } = unsafe { &mut *ptr.0 }.op.take().unwrap();
        ti.push(callback);
    }
}

pub struct TimerQueue {
    mutex: Mutex<Vec<EntryPtr>>
}

impl TimerQueue {
    pub fn new() -> TimerQueue {
        TimerQueue {
            mutex: Mutex::new(Vec::new()),
        }
    }

    fn set(&self, ptr: *mut Entry, mut op: Op, is_first: &mut bool) -> Option<Callback> {
        let mut queue = self.mutex.lock().unwrap();
        if let Some(old_op) = unsafe { &mut *ptr }.op.as_mut() {
            remove(&mut queue, ptr);
            mem::swap(old_op, &mut op);
            *is_first = insert(&mut queue, ptr);
            Some(op.callback)
        } else {
            unsafe { &mut *ptr }.op = Some(op);
            *is_first = insert(&mut queue, ptr);
            None
        }
    }

    fn unset(&self, ptr: *mut Entry, expiry_opt: &mut Option<Expiry>) -> Option<Callback> {
        let mut queue = self.mutex.lock().unwrap();
        if let Some(_) = unsafe { &mut *ptr }.op.as_mut() {
            if remove(&mut queue, ptr) {
                *expiry_opt = Some(
                    if let Some(ptr) = queue.first() {
                        unsafe { &*ptr.0 }.op.as_ref().unwrap().expiry
                    } else {
                        Default::default()
                    }
                )
            }
            let Op { expiry:_, callback } = unsafe { &mut *ptr }.op.take().unwrap();
            Some(callback)
        } else {
            None
        }
    }

    pub fn cancel_all(&self, ti: &ThreadInfo) {
        let mut queue = self.mutex.lock().unwrap();
        let len = queue.len();
        drain(&mut queue, len, ti);
    }

    pub fn ready_expired(&self, ti: &ThreadInfo) -> usize {
        let mut queue = self.mutex.lock().unwrap();
        let len = find_timeout(&queue, Expiry::now());
        drain(&mut queue, len, ti);
        queue.len()
    }
}

pub struct TimerActor {
    io: IoService,
    ptr: UnsafeBoxedCell<Entry>,
}

impl TimerActor {
    pub fn new(io: &IoService) -> TimerActor {
        TimerActor {
            io: io.clone(),
            ptr: UnsafeBoxedCell::new(Entry { op: None }),
        }
    }

    pub fn set_wait(&self, expiry: Expiry, callback: Callback) {
        let mut is_first = false;
        let op = Op { expiry: expiry, callback: callback };
        if let Some(callback) = self.io.0.queue.set(unsafe { self.ptr.get() }, op, &mut is_first) {
            self.io.post(|io| callback(io, ECANCELED));
        }
        if is_first {
            self.io.0.ctrl.reset_timeout(expiry)
        }
    }

    pub fn unset_wait(&self) -> Option<Callback> {
        let mut expiry_opt = None;
        let callback_opt = self.io.0.queue.unset(unsafe { self.ptr.get() }, &mut expiry_opt);
        if let Some(expiry) = expiry_opt {
            self.io.0.ctrl.reset_timeout(expiry);
        }
        callback_opt
    }
}

unsafe impl IoObject for TimerActor {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}

#[test]
fn test_timer_set_unset() {
    let io = &IoService::new();
    let act = TimerActor::new(io);
    act.set_wait(Expiry::now(), Box::new(|_,_| {}));
    assert!(act.unset_wait().is_some());
}

#[test]
fn test_timer_set_ready() {
    let io = &IoService::new();

    let act1 = TimerActor::new(io);
    act1.set_wait(Expiry::now(), Box::new(|_,_| {}));

    let act2 = TimerActor::new(io);
    act2.set_wait(Expiry::default(), Box::new(|_,_| {}));

    let ti = ThreadInfo::new().unwrap();
    io.0.queue.ready_expired(&ti);
    assert_eq!(ti.collect().len(), 1);
}

#[test]
fn test_timer_set_cancel() {
    let io = &IoService::new();

    let act1 = TimerActor::new(io);
    act1.set_wait(Expiry::now(), Box::new(|_,_| {}));

    let act2 = TimerActor::new(io);
    act2.set_wait(Expiry::default(), Box::new(|_,_| {}));

    let ti = ThreadInfo::new().unwrap();
    io.0.queue.cancel_all(&ti);
    assert_eq!(ti.collect().len(), 2);
}
