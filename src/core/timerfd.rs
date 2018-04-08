use ffi::{AsRawFd, SystemError};
use core::{Perform, ThreadIoContext, Expiry, TimerImpl, UnsafeRef, Handle, Reactor};

use std::sync::Mutex;
use libc::{timerfd_create, timerfd_settime, timespec, itimerspec, TFD_TIMER_ABSTIME,
           CLOCK_MONOTONIC, TFD_NONBLOCK, TFD_CLOEXEC};

type TimerRef = UnsafeRef<TimerImpl>;

pub struct TimerFdQueue {
    mutex: Mutex<Vec<TimerRef>>,
    timerfd: Handle,
}

impl TimerFdQueue {
    pub fn new() -> Result<Self, SystemError> {
        match unsafe { timerfd_create(CLOCK_MONOTONIC, TFD_NONBLOCK | TFD_CLOEXEC) } {
            -1 => Err(SystemError::last_error()),
            fd => Ok(TimerFdQueue {
                mutex: Mutex::default(),
                timerfd: Handle::intr(fd),
            }),
        }
    }

    pub fn startup(&self, reactor: &Reactor) {
        reactor.register_intr(&self.timerfd);
    }

    pub fn cleanup(&self, reactor: &Reactor) {
        reactor.deregister_intr(&self.timerfd)
    }

    fn set_time(&self, expiry: &Expiry) {
        use std::ptr;

        let iti = itimerspec {
            it_interval: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: expiry.abs_time(),
        };
        unsafe {
            timerfd_settime(
                self.timerfd.as_raw_fd(),
                TFD_TIMER_ABSTIME,
                &iti,
                ptr::null_mut(),
            );
        }
    }

    pub fn wait_duration(&self, max: usize) -> usize {
        max
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

    pub fn insert(&self, timer: &TimerImpl, op: Box<Perform>) -> Option<Box<Perform>> {
        let mut tq = self.mutex.lock().unwrap();
        let mut timer = TimerRef::new(timer);
        let old_op = timer.op.take();
        timer.op = Some(op);
        let i = tq.binary_search(&timer).unwrap_err();
        tq.insert(i, unsafe { timer.clone() });
        if i == 0 {
            self.set_time(&timer.expiry);
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
                self.set_time(&timer.expiry);
            }
        }
        timer.expiry = expiry;
        old_op
    }
}
