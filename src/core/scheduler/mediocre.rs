use super::TimerContext;
use unsafe_cell::UnsafeBoxedCell;
use core::{Reactor, ThreadIoContext, Expiry, TimerQueue, Operation};

use std::io;
use std::cmp;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct MediocreScheduler {
    mutex: Mutex<TimerQueue>,
    outstanding_work: Arc<AtomicUsize>,
}

impl MediocreScheduler {
    pub fn new(outstanding_work: Arc<AtomicUsize>) -> io::Result<Self> {
        Ok(MediocreScheduler{
            mutex: Default::default(),
            outstanding_work: outstanding_work,
        })
    }

    pub fn startup(&self, _: &Reactor) {
    }

    pub fn cleanup(&self, _: &Reactor) {
    }

    pub fn wait_duration(&self, max: Duration) -> Duration {
        if let Some(expiry) = self.mutex.lock().unwrap().front() {
            cmp::min(max, expiry.left())
        } else {
            max
        }
    }

    pub fn timer_queue_insert(&self, mut timer: UnsafeBoxedCell<TimerContext>, op: Operation) -> Option<Operation> {
        let (old_op, update) = {
            let mut tq = self.mutex.lock().unwrap();
            let old_op = timer.op.take();
            timer.op = Some(op);
            (old_op, tq.insert(&timer).is_some())
        };

        if update {
            timer.ctx.0.interrupter.interrupt();
        }

        if old_op.is_none() {
            self.outstanding_work.fetch_add(1, Ordering::SeqCst);
        }
        old_op
    }

    pub fn timer_queue_remove(&self, mut timer: UnsafeBoxedCell<TimerContext>) -> Option<Operation> {
        let mut tq = self.mutex.lock().unwrap();
        let old_op = timer.op.take();
        let _ = tq.erase(&timer);
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
        self.mutex.lock().unwrap().get_ready_timers(this, Expiry::now());
        self.outstanding_work.fetch_sub(this.len() - len, Ordering::SeqCst);
    }
}
