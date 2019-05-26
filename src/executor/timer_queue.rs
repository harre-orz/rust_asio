//

use super::{Interrupter, Reactor, YieldContext};
use std::cmp::Ordering;
use std::ptr::NonNull;
use std::time::Instant;

struct YieldContextRef(NonNull<YieldContext>);

impl Eq for YieldContextRef {}

impl Ord for YieldContextRef {
    fn cmp(&self, other: &Self) -> Ordering {
        match unsafe { self.0.as_ref().expiry.cmp(&other.0.as_ref().expiry) } {
            Ordering::Equal => self.0.as_ptr().cmp(&other.0.as_ptr()),
            cmp => cmp,
        }
    }
}

impl PartialEq for YieldContextRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

impl PartialOrd for YieldContextRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct TimerQueue {
    stable_set: Vec<YieldContextRef>,
}

impl TimerQueue {
    pub fn new() -> Self {
        TimerQueue { stable_set: Vec::new() }
    }

    // locked_A, locked_B
    pub fn insert(&mut self, yield_ctx: &mut YieldContext, intr: &mut Interrupter) {
        let yield_ref = YieldContextRef(unsafe { NonNull::new_unchecked(yield_ctx) });
        let i = self.stable_set.binary_search(&yield_ref).unwrap_err();
        self.stable_set.insert(i, yield_ref);
        if i == 0 {
            intr.reset_timeout(yield_ctx.expiry);
        }
    }

    // locked_A, locked_B
    pub fn erase(&mut self, yield_ctx: &mut YieldContext, intr: &mut Interrupter) {
        let yield_ref = YieldContextRef(unsafe { NonNull::new_unchecked(yield_ctx) });
        if let Ok(i) = self.stable_set.binary_search(&yield_ref) {
            self.stable_set.remove(i);
            if let Some(yield_ref) = self.stable_set.first() {
                intr.reset_timeout(unsafe { yield_ref.0.as_ref() }.expiry);
            }
        }
    }

    pub fn get_ready_timers(&mut self, reactor: &Reactor) {
        let now = Instant::now();
        reactor.mutex.lock();
        let i = match self
            .stable_set
            .binary_search_by(|yield_ref| unsafe { yield_ref.0.as_ref().expiry.cmp(&now) })
        {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        for mut yield_ref in self.stable_set.drain(..i) {
            unsafe { yield_ref.0.as_mut() }.consume(reactor);
        }
        reactor.mutex.unlock();
    }
}
