use ffi::SystemError;
use core::{Perform, ThreadIoContext, Expiry, TimerImpl, UnsafeRef};

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

type TimerRef = UnsafeRef<TimerImpl>;

pub struct SimpleTimerQueue {
    mutex: Mutex<Vec<TimerRef>>,
    timeout_nsec: AtomicUsize,
}

impl SimpleTimerQueue {
    pub fn new() -> Self {
        SimpleTimerQueue {
            mutex: Mutex::default(),
            timeout_nsec: AtomicUsize::new(0),
        }
    }

    pub fn wait_duration(&self, max: usize) -> usize {
        use std::cmp;
        cmp::min(self.timeout_nsec.load(Ordering::Relaxed), max)
    }

    pub fn get_ready_timers(&self, this: &mut ThreadIoContext) {
        let mut tq = self.mutex.lock().unwrap();
        let i = match tq.binary_search_by(|e| e.expiry.cmp(&Expiry::now())) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        for mut e in tq.drain(..i) {
            this.push(e.op.take().unwrap(), SystemError::default());
        }
    }

    pub fn insert(&self, timer: &TimerImpl, op: Box<Perform>)  -> Option<Box<Perform>> {
        let mut tq = self.mutex.lock().unwrap();
        let mut timer = TimerRef::new(timer);
        let old_op = timer.op.take();
        timer.op = Some(op);
        let i = tq.binary_search(&timer).unwrap_err();
        tq.insert(i, unsafe { timer.clone() });
        if i == 0 {
            self.timeout_nsec.store(timer.expiry.left(), Ordering::SeqCst);
            timer.ctx.as_reactor().interrupt();
        }
        old_op
    }

    pub fn erase(&self, timer: &TimerImpl, expiry: Expiry) -> Option<Box<Perform>> {
        let mut tq = self.mutex.lock().unwrap();
        let mut timer = TimerRef::new(timer);
        let old_op = timer.op.take();
        if let Ok(i) = tq.binary_search(&timer) {
            tq.remove(i);
            for timer in tq.first().iter() {
                self.timeout_nsec.store(timer.expiry.left(), Ordering::SeqCst);
                timer.ctx.as_reactor().interrupt();
            }
        }
        timer.expiry = expiry;
        old_op
    }
}
