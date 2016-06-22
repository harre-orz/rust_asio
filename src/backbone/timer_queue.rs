use std::mem;
use std::cmp::{Eq, PartialEq, Ord, PartialOrd, Ordering};
use std::cell::UnsafeCell;
use std::boxed::FnBox;
use std::sync::Mutex;
use {IoService};
use super::{Handler, Expiry, HandlerResult};

struct TimerOp {
    expiry: Expiry,
    id: usize,
    callback: Handler,
}

type HandlerCPtr = *const FnBox(HandlerResult);

impl Eq for TimerOp {
}

impl PartialEq for TimerOp {
    fn eq(&self, other: &Self) -> bool {
        let lhs: HandlerCPtr = &*self.callback;
        let rhs: HandlerCPtr = &*other.callback;
        self.expiry.eq(&other.expiry) && lhs.eq(&rhs)
    }
}

impl Ord for TimerOp {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.expiry.cmp(&other.expiry) {
            Ordering::Equal => {
                let lhs: HandlerCPtr = &*self.callback;
                let rhs: HandlerCPtr = &*other.callback;
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
                let lhs: HandlerCPtr = &*self.callback;
                let rhs: HandlerCPtr = &*other.callback;
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

    fn do_set_timer(&self, ptr: *mut TimerObject, mut timer_op: TimerOp) -> Option<Handler> {
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
    timer_ptr: UnsafeCell<TimerObject>,
}

impl TimerActor {
    pub fn new() -> TimerActor {
        TimerActor {
            timer_ptr: UnsafeCell::new(TimerObject {
                timer_op: None,
            }),
        }
    }

    pub fn set_timer(&self, io: &IoService, expiry: Expiry, id: usize, callback: Handler) {
        let ptr = unsafe { &mut *self.timer_ptr.get() };
        let timer = &io.0.queue;
        if let Some(callback) = timer.do_set_timer(ptr, TimerOp { expiry: expiry, id: id, callback: callback }) {
            io.0.task.post(id, Box::new(move || callback(HandlerResult::Canceled)));

        }
        io.0.reset_timeout(timer.first_timeout());
    }

    pub fn unset_timer(&self, io: &IoService) -> Option<Handler> {
        let ptr = unsafe { &mut *self.timer_ptr.get() };
        let timer = &io.0.queue;
        timer.do_unset_timer(ptr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::TimerOp;
    use super::super::ToExpiry;
    use time;
    use std::thread;
    use {IoService, Strand};

    #[test]
    fn test_timer_set_unset() {
        let io = IoService::new();
        let mut ev = Strand::new(&io, TimerActor::new());
        assert!(io.0.queue.do_unset_timer(unsafe { &mut *ev.timer_ptr.get() }).is_none());
        assert!(io.0.queue.do_set_timer(unsafe { &mut *ev.timer_ptr.get() }, TimerOp {
            expiry: time::SteadyTime::now().to_expiry(),
            id: 0,
            callback: Box::new(|_| {})
        }).is_none());
        assert!(io.0.queue.do_set_timer(unsafe { &mut *ev.timer_ptr.get() }, TimerOp {
            expiry: time::SteadyTime::now().to_expiry(),
            id: 0,
            callback: Box::new(|_| {})
        }).is_some());
        let arc = ev.0.clone();
        thread::spawn(move || {
            let mut ev = Strand(arc);
            assert!(io.0.queue.do_unset_timer(unsafe { &mut *ev.timer_ptr.get() }).is_some());
            assert!(io.0.queue.do_unset_timer(unsafe { &mut *ev.timer_ptr.get() }).is_none());
        }).join().unwrap();
    }


    #[test]
    fn test_ordered_queue() {
        let io = IoService::new();
        let sv: &TimerQueue = &io.0.queue;
        let ev1 = TimerActor::new();
        let ev2 = TimerActor::new();
        let ev3 = TimerActor::new();
        let now = time::SteadyTime::now();
        ev1.set_timer(&io, (now + time::Duration::minutes(1)).to_expiry(), 0, Box::new(|_| {}));
        ev2.set_timer(&io, now.to_expiry(), 0, Box::new(|_| {}));
        assert!(sv.first_timeout() == now.to_expiry());
        ev3.set_timer(&io, (now - time::Duration::seconds(1)).to_expiry(), 0, Box::new(|_| {}));
        assert!(sv.first_timeout() == (now - time::Duration::seconds(1)).to_expiry());
        let _ = ev2.unset_timer(&io);
        let mut vec = Vec::new();
        sv.drain_expired(&mut vec);
        assert!(vec.len() == 1);
        assert!(sv.first_timeout() == (now + time::Duration::minutes(1)).to_expiry());
        let _ = ev1.unset_timer(&io);
    }

    #[test]
    #[should_panic]
    fn test_timer_panic() {
        let io1 = IoService::new();
        let io2 = IoService::new();
        let ev = TimerActor::new();
        ev.set_timer(&io1, time::now().to_expiry(), 0, Box::new(|_| {}));
        ev.unset_timer(&io2);
    }
}
