use std::cmp;
use std::mem;
use std::sync::Mutex;
use std::time::Duration;
use time;
use {IoObject, IoService};
use super::{ErrorCode, Handler, READY, CANCELED};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Expiry(Duration);

impl Expiry {
    pub fn wait_duration(&self) -> Duration {
        self.0
    }
}

impl Default for Expiry {
    fn default() -> Expiry {
        Expiry(Duration::new(0, 0))
    }
}

pub trait ToExpiry {
    fn zero() -> Self;
    fn now() -> Self;
    fn to_expiry(&self) -> Expiry;
}

impl ToExpiry for time::SteadyTime {
    fn zero() -> Self {
        unsafe { mem::zeroed() }
    }

    fn now() -> Self {
        time::SteadyTime::now()
    }

    fn to_expiry(&self) -> Expiry {
        match (*self - Self::zero()).to_std() {
            Ok(expiry) => Expiry(expiry),
            _ => Expiry::default(),
        }
    }
}

impl ToExpiry for time::Timespec {
    fn zero() -> Self {
        time::Timespec::new(0, 0)
    }

    fn now() -> Self {
        time::get_time()
    }

    fn to_expiry(&self) -> Expiry {
        match ((time::SteadyTime::now() - time::SteadyTime::zero()) + (*self - Self::now())).to_std() {
            Ok(expiry) => Expiry(expiry),
            _ => Expiry::default(),
        }
    }
}


struct TimerOp {
    expiry: Expiry,
    handler: Handler,
}

struct TimerData {
    operation: Option<TimerOp>
}

struct TimerEntry(*mut TimerData);

impl cmp::Eq for TimerEntry {}

impl cmp::PartialEq for TimerEntry {
    fn eq(&self, other: &TimerEntry) -> bool {
        self.0 == other.0
    }
}

impl cmp::Ord for TimerEntry {
    fn cmp(&self, other: &TimerEntry) -> cmp::Ordering {
        let lhs = unsafe { &*self.0 };
        let rhs = unsafe { &*other.0 };

        if self.0 == other.0 {
            cmp::Ordering::Equal
        } else if lhs.operation.as_ref().unwrap().expiry < rhs.operation.as_ref().unwrap().expiry {
            cmp::Ordering::Less
        } else if lhs.operation.as_ref().unwrap().expiry > rhs.operation.as_ref().unwrap().expiry {
            cmp::Ordering::Greater
        } else if self.0 < other.0 {
            cmp::Ordering::Less
        } else {
            cmp::Ordering::Greater
        }
    }
}

impl cmp::PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &TimerEntry) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

unsafe impl Send for TimerEntry {}

unsafe impl Sync for TimerEntry {}


pub struct TimerQueue {
    mutex: Mutex<Vec<TimerEntry>>,
}

impl TimerQueue {
    pub fn new() -> TimerQueue {
        TimerQueue {
            mutex: Mutex::new(Vec::new()),
        }
    }

    fn insert(queue: &mut Vec<TimerEntry>, ptr: *mut TimerData) -> bool {
        debug_assert!(unsafe { &*ptr }.operation.is_some());

        let key = TimerEntry(ptr);
        let idx = match queue.binary_search(&key) {
            Ok(len) => len + 1,
            Err(len) => len,
        };
        queue.insert(idx, key);
        idx == 0
    }

    fn remove(queue: &mut Vec<TimerEntry>, ptr: *mut TimerData) -> bool {
        debug_assert!(unsafe { &*ptr }.operation.is_some());

        let key = TimerEntry(ptr);
        let idx = queue.binary_search(&key).unwrap();
        queue.remove(idx);
        idx == 0
    }

    fn set(&self, ptr: *mut TimerData, mut op: TimerOp, is_first: &mut bool) -> Option<Handler> {
        let mut queue = self.mutex.lock().unwrap();
        if let Some(old_op) = unsafe { &mut *ptr }.operation.as_mut() {
            Self::remove(&mut queue, ptr);
            mem::swap(old_op, &mut op);
            *is_first = Self::insert(&mut queue, ptr);
            Some(op.handler)
        } else {
            unsafe { &mut *ptr }.operation = Some(op);
            *is_first = Self::insert(&mut queue, ptr);
            None
        }
    }

    fn unset(&self, ptr: *mut TimerData, expiry: &mut Option<Expiry>) -> Option<Handler> {
        let mut queue = self.mutex.lock().unwrap();
        if let Some(_) = unsafe { &mut *ptr }.operation.as_mut() {
            if Self::remove(&mut queue, ptr) {
                *expiry = Some(
                    if let Some(e) = queue.first() {
                        unsafe { &*e.0 }.operation.as_ref().unwrap().expiry
                    } else {
                        (time::SteadyTime::now() + time::Duration::seconds(60 * 5)).to_expiry()
                    });
            }
            let TimerOp { expiry:_, handler } = unsafe { &mut *ptr }.operation.take().unwrap();
            Some(handler)
        } else {
            None
        }
    }

    fn find_timeout(queue: &Vec<TimerEntry>, expiry: Expiry) -> usize {
        for (i, e) in queue.iter().enumerate() {
            if unsafe { &*e.0 }.operation.as_ref().unwrap().expiry > expiry {
                return i;
            }
        }
        queue.len()
    }

    fn cancel(queue: &mut Vec<TimerEntry>, len: usize, io: &IoService, ec: ErrorCode) {
        for e in queue.drain(..len) {
            let e = unsafe { &mut *e.0 };
            let TimerOp { expiry:_, handler } = e.operation.take().unwrap();
            io.post(move |io| handler(io, ec));
        }
    }

    pub fn cancel_all(&self, io: &IoService) {
        let mut queue = self.mutex.lock().unwrap();
        let len = queue.len();
        Self::cancel(&mut queue, len, io, ErrorCode(CANCELED));
    }

    pub fn cancel_expired(&self, io: &IoService) -> usize {
        let mut queue = self.mutex.lock().unwrap();
        let len = Self::find_timeout(&queue, time::SteadyTime::now().to_expiry());
        Self::cancel(&mut queue, len, io, ErrorCode(READY));
        queue.len()
    }
}


pub struct WaitActor {
    io: IoService,
    timer_ptr: *mut TimerData,
}

impl WaitActor {
    pub fn new<T: IoObject>(io: &T) -> WaitActor {
        WaitActor {
            io: io.io_service().clone(),
            timer_ptr: Box::into_raw(Box::new(TimerData { operation: None })),
        }
    }

    pub fn set_wait(&self, expiry: Expiry, handler: Handler) {
        let mut is_first = false;
        if let Some(handler) = self.io.0.queue.set(self.timer_ptr, TimerOp { expiry: expiry, handler: handler }, &mut is_first) {
            self.io.post(|io| handler(io, ErrorCode(CANCELED)));
        }
        if is_first {
            self.io.0.ctrl.reset_timeout(expiry);
        }
    }

    pub fn unset_wait(&self) -> Option<Handler> {
        let mut expiry = None;
        let res = self.io.0.queue.unset(self.timer_ptr, &mut expiry);
        if let Some(expiry) = expiry {
            self.io.0.ctrl.reset_timeout(expiry);
        }
        res
    }
}

impl IoObject for WaitActor {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}

impl Drop for WaitActor {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.timer_ptr) };
    }
}

unsafe impl Send for WaitActor {}

unsafe impl Sync for WaitActor {}

#[cfg(test)]
mod tests {
    use super::*;
    use super::TimerOp;
    use test::Bencher;
    use time;
    use IoService;
    use std::thread;

    #[bench]
    fn bench_system_time_now(b: &mut Bencher) {
        b.iter(|| {
            let _ = time::get_time();
        });
    }

    #[bench]
    fn bench_system_time_to_expiry(b: &mut Bencher) {
        let t = time::get_time();
        b.iter(|| {
            let _ = t.to_expiry();
        });
    }

    #[bench]
    fn bench_steady_time_now(b: &mut Bencher) {
        b.iter(|| {
            let _ = time::SteadyTime::now();
        });
    }

    #[bench]
    fn bench_steady_time_to_expiry(b: &mut Bencher) {
        let t = time::SteadyTime::now();
        b.iter(|| {
            let _ = t.to_expiry();
        });
    }

    pub fn first_timeout(queue: &TimerQueue) -> Expiry {
        let queue = queue.mutex.lock().unwrap();
        let e = queue.first().unwrap();
        unsafe { &*e.0 }.operation.as_ref().unwrap().expiry
    }

    #[test]
    fn test_wait_set_unset() {
        let io = &IoService::new();
        let ev = WaitActor::new(io);
        let mut is_first = false;
        let mut expiry = None;
        assert!(io.0.queue.unset(unsafe { &mut *ev.timer_ptr }, &mut expiry).is_none());
        assert!(expiry.is_none());
        assert!(io.0.queue.set(unsafe { &mut *ev.timer_ptr }, TimerOp {
            expiry: time::SteadyTime::now().to_expiry(),
            handler: Box::new(|_,_| {})
        }, &mut is_first).is_none());
        assert!(is_first);
        assert!(io.0.queue.set(unsafe { &mut *ev.timer_ptr }, TimerOp {
            expiry: time::SteadyTime::now().to_expiry(),
            handler: Box::new(|_,_| {})
        }, &mut is_first).is_some());
        assert!(is_first);
        let io = io.clone();
        thread::spawn(move || {
            let mut expiry = None;
            assert!(io.0.queue.unset(unsafe { &mut *ev.timer_ptr }, &mut expiry).is_some());
            assert!(expiry.is_some());
            let mut expiry = None;
            assert!(io.0.queue.unset(unsafe { &mut *ev.timer_ptr }, &mut expiry).is_none());
            assert!(expiry.is_none());
        }).join().unwrap();
    }

    #[test]
    fn test_ordered_queue() {
        let io = &IoService::new();
        let sv: &TimerQueue = &io.0.queue;
        let ev1 = WaitActor::new(io);
        let ev2 = WaitActor::new(io);
        let ev3 = WaitActor::new(io);
        let now = time::SteadyTime::now();
        ev1.set_wait((now + time::Duration::minutes(1)).to_expiry(), Box::new(|_,_| {}));
        ev2.set_wait(now.to_expiry(), Box::new(|_,_| {}));
        assert!(first_timeout(sv) == now.to_expiry());
        ev3.set_wait((now - time::Duration::seconds(1)).to_expiry(), Box::new(|_,_| {}));
        assert!(first_timeout(sv) == (now - time::Duration::seconds(1)).to_expiry());
        let _ = ev2.unset_wait();
        sv.cancel_expired(io);
        assert!(first_timeout(sv) == (now + time::Duration::minutes(1)).to_expiry());
        let _ = ev1.unset_wait();
    }

    #[bench]
    fn bench_timer_set(b: &mut Bencher) {
        let io = &IoService::new();
        let ev = WaitActor::new(io);
        let expiry = time::get_time().to_expiry();
        b.iter(|| {
            ev.set_wait(expiry, Box::new(|_,_| {}));
        });
    }

    #[bench]
    fn bench_timer_set_unset(b: &mut Bencher) {
        let io = &IoService::new();
        let ev = WaitActor::new(io);
        let expiry = time::get_time().to_expiry();
        b.iter(|| {
            ev.set_wait(expiry, Box::new(|_,_| {}));
            ev.unset_wait();
        });
    }
}
