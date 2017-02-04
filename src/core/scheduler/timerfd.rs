use unsafe_cell::UnsafeBoxedCell;
use ffi::{AsRawFd};
use core::{Reactor, ThreadIoContext, TimerContext, Expiry, TimerQueue, Operation, IntrFd};

use std::io;
use std::ptr;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use libc::{c_int, timespec, CLOCK_MONOTONIC};

#[repr(C)]
pub struct itimerspec {
    pub it_interval: timespec,
    pub it_value: timespec,
}

const TFD_CLOEXEC: c_int = 0o2000000;
const TFD_NONBLOCK: c_int = 0o4000;
const TFD_TIMER_ABSTIME: c_int = 1 << 0;

extern {
    fn timerfd_create(
        clkid: c_int,
        flags: c_int
    ) -> c_int;

    fn timerfd_settime(
        fd: c_int,
        flags: c_int,
        new_value: *const itimerspec,
        old_value: *mut itimerspec
    ) -> c_int;

    fn timerfd_gettime(
        fd: c_int,
        curr_value:
        *mut itimerspec
    ) -> c_int;
}

pub struct TimerFdScheduler {
    tfd: IntrFd,
    mutex: Mutex<TimerQueue>,
    outstanding_work: Arc<AtomicUsize>,
}

impl TimerFdScheduler {
    pub fn new(outstanding_work: Arc<AtomicUsize>) -> io::Result<Self> {
        let tfd = libc_try!(timerfd_create(CLOCK_MONOTONIC, TFD_CLOEXEC));
        Ok(TimerFdScheduler {
            tfd: IntrFd::new::<Self>(tfd),
            mutex: Default::default(),
            outstanding_work: outstanding_work,
        })
    }

    pub fn startup(&self, ctx: &Reactor) {
        ctx.register_intr_fd(&self.tfd)
    }

    pub fn cleanup(&self, ctx: &Reactor) {
        ctx.deregister_intr_fd(&self.tfd)
    }

    pub fn wait_duration(&self, max: Duration) -> Duration {
        max
    }

    pub fn timer_queue_insert(&self, mut timer: UnsafeBoxedCell<TimerContext>, op: Operation) -> Option<Operation> {
        let (old_op, expiry) = {
            let mut tq = self.mutex.lock().unwrap();
            let old_op = timer.op.take();
            timer.op = Some(op);
            (old_op, tq.insert(&timer))
        };

        if let Some(expiry) = expiry {
            do_reset_timeout(&self.tfd, expiry)
        }

        if old_op.is_none() {
            self.outstanding_work.fetch_add(1, Ordering::SeqCst);
        }
        old_op
    }

    pub fn timer_queue_remove(&self, mut timer: UnsafeBoxedCell<TimerContext>) -> Option<Operation> {
        let mut tq = self.mutex.lock().unwrap();
        let old_op = timer.op.take();
        if let Some(expiry) = tq.erase(&timer) {
            do_reset_timeout(&self.tfd, expiry);
        }
        if old_op.is_some() {
            self.outstanding_work.fetch_sub(1, Ordering::SeqCst);
        }
        old_op
    }

    pub fn cancel_all_timers(&self, this: &mut ThreadIoContext) {
        self.mutex.lock().unwrap().cancel_all_timers(this)
    }

    pub fn get_ready_timers(&self, this: &mut ThreadIoContext) {
        let len = this.len();
        if let Some(expiry) = {
            let mut tq = self.mutex.lock().unwrap();
            tq.get_ready_timers(this, Expiry::now());
            tq.front()
        } {
            do_reset_timeout(&self.tfd, expiry);
        }
        self.outstanding_work.fetch_sub(this.len() - len, Ordering::SeqCst);
    }
}

fn do_reset_timeout<T>(t: &T, expiry: Expiry)
    where T: AsRawFd,
{
    let expiry = expiry.abs();
    let new_value = itimerspec {
        it_interval: timespec { tv_sec: 0, tv_nsec: 0 },
        it_value: timespec {
            tv_sec: expiry.as_secs() as i64,
            tv_nsec: expiry.subsec_nanos() as i64,
        },
    };
    libc_ign!(timerfd_settime(
        t.as_raw_fd(),
        TFD_TIMER_ABSTIME,
        &new_value, ptr::null_mut()
    ));
}
