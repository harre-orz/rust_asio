use std::mem;
use std::cmp::{Eq, PartialEq, Ord, PartialOrd, Ordering};
use std::cell::UnsafeCell;
use std::boxed::FnBox;
use std::sync::Mutex;
use {IoObject, IoService};
use super::{Handler, Expiry, HandlerResult, TaskExecutor};

struct TimerOp {
    expiry: Expiry,
    id: usize,
    callback: Handler,
}

type HandlerCPtr = *const FnBox(*const IoService, HandlerResult);

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

    fn do_unset_timer(&self, ptr: *mut TimerObject) -> Option<(usize, Handler)> {
        if let Some(old_op) = {
            let mut timer_op = None;
            let mut queue = self.mutex.lock().unwrap();
            if let Some(_) = unsafe { &mut *ptr }.timer_op.as_mut() {
                Self::remove(&mut queue, ptr);
            }
            mem::swap(&mut unsafe { &mut *ptr }.timer_op, &mut timer_op);
            timer_op
        } {
            Some((old_op.id, old_op.callback))
        } else {
            None
        }
    }

    pub fn drain_all(&self, task: &TaskExecutor) -> usize {
        let mut queue = self.mutex.lock().unwrap();
        let len = queue.len();
        Self::drain(&mut queue, len, task);
        0
    }

    pub fn drain_expired(&self, task: &TaskExecutor) -> usize {
        let mut queue = self.mutex.lock().unwrap();
        let len = Self::find_timeout(&queue, &Expiry::now());
        Self::drain(&mut queue, len, task);
        queue.len()
    }

    fn drain(queue: &mut Vec<TimerEntry>, len: usize, task: &TaskExecutor) {
        for e in queue.drain(..len) {
            let mut timer_op = None;
            mem::swap(&mut unsafe { &mut *e.ptr }.timer_op, &mut timer_op);
            let TimerOp { expiry:_, id, callback } = timer_op.unwrap();
            task.post(id, Box::new(move |io| callback(io, HandlerResult::Ready)));
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
    timer_ptr: UnsafeCell<TimerObject>,
}

impl TimerActor {
    pub fn new<T: IoObject>(io: &T) -> TimerActor {
        TimerActor {
            io: io.io_service().clone(),
            timer_ptr: UnsafeCell::new(TimerObject {
                timer_op: None,
            }),
        }
    }

    pub fn set_timer(&self, expiry: Expiry, id: usize, callback: Handler) {
        let ptr = unsafe { &mut *self.timer_ptr.get() };
        let timer = &self.io.0.queue;
        if let Some(callback) = timer.do_set_timer(ptr, TimerOp { expiry: expiry, id: id, callback: callback }) {
            self.io.0.task.post(id, Box::new(move |io| callback(io, HandlerResult::Canceled)));

        }
        self.io.0.reset_timeout(timer.first_timeout());
    }

    pub fn unset_timer(&self) -> Option<(usize, Handler)> {
        let ptr = unsafe { &mut *self.timer_ptr.get() };
        let timer = &self.io.0.queue;
        timer.do_unset_timer(ptr)
    }
}

impl IoObject for TimerActor {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}

#[cfg(test)]
mod tests {
    use {IoService, Strand};
    use super::*;
    use super::TimerOp;
    use super::super::ToExpiry;
    use time;
    use std::thread;
    use test::Bencher;

    #[test]
    fn test_timer_set_unset() {
        let io = IoService::new();
        let mut ev = Strand::new(&io, TimerActor::new(&io));
        assert!(io.0.queue.do_unset_timer(unsafe { &mut *ev.timer_ptr.get() }).is_none());
        assert!(io.0.queue.do_set_timer(unsafe { &mut *ev.timer_ptr.get() }, TimerOp {
            expiry: time::SteadyTime::now().to_expiry(),
            id: 0,
            callback: Box::new(|_,_| {})
        }).is_none());
        assert!(io.0.queue.do_set_timer(unsafe { &mut *ev.timer_ptr.get() }, TimerOp {
            expiry: time::SteadyTime::now().to_expiry(),
            id: 0,
            callback: Box::new(|_,_| {})
        }).is_some());
        let obj = ev.obj.clone();
        let io = io.clone();
        thread::spawn(move || {
            let mut ev = Strand { io: &io, obj: obj };
            assert!(io.0.queue.do_unset_timer(unsafe { &mut *ev.timer_ptr.get() }).is_some());
            assert!(io.0.queue.do_unset_timer(unsafe { &mut *ev.timer_ptr.get() }).is_none());
        }).join().unwrap();
    }


    #[test]
    fn test_ordered_queue() {
        let io = IoService::new();
        let sv: &TimerQueue = &io.0.queue;
        let ev1 = TimerActor::new(&io);
        let ev2 = TimerActor::new(&io);
        let ev3 = TimerActor::new(&io);
        let now = time::SteadyTime::now();
        ev1.set_timer((now + time::Duration::minutes(1)).to_expiry(), 0, Box::new(|_,_| {}));
        ev2.set_timer(now.to_expiry(), 0, Box::new(|_,_| {}));
        assert!(sv.first_timeout() == now.to_expiry());
        ev3.set_timer((now - time::Duration::seconds(1)).to_expiry(), 0, Box::new(|_,_| {}));
        assert!(sv.first_timeout() == (now - time::Duration::seconds(1)).to_expiry());
        let _ = ev2.unset_timer();
        sv.drain_expired(&io.0.task);
        assert!(sv.first_timeout() == (now + time::Duration::minutes(1)).to_expiry());
        let _ = ev1.unset_timer();
    }

    #[bench]
    fn bench_timer_set(b: &mut Bencher) {
        let io = IoService::new();
        let ev = Strand::new(&io, TimerActor::new(&io));
        b.iter(|| {
            ev.set_timer(time::now().to_expiry(), 0, Box::new(|_,_| {}));
        });
    }

    #[bench]
    fn bench_timer_set_unset(b: &mut Bencher) {
        let io = IoService::new();
        let ev = Strand::new(&io, TimerActor::new(&io));
        b.iter(|| {
            ev.set_timer(time::now().to_expiry(), 0, Box::new(|_,_| {}));
            ev.unset_timer();
        });
    }
}
