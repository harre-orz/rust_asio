use core::{IoContext, AsIoContext, ThreadIoContext, Expiry, TimerOp};

use std::marker::PhantomData;
use std::ops::Add;
use std::time::{Duration, SystemTime, Instant};

pub trait Clock : Send + 'static {
    type Duration;

    type TimePoint : Add<Self::Duration, Output = Self::TimePoint> + Into<Expiry>;

    fn now() -> Self::TimePoint;
}

/// Provides a monotonic clock.
pub struct SteadyClock;

impl Clock for SteadyClock {
    type Duration = Duration;

    type TimePoint = Instant;

    fn now() -> Self::TimePoint {
        Instant::now()
    }
}

/// Provides a real-time clock.
pub struct SystemClock;

impl Clock for SystemClock {
    type Duration = Duration;

    type TimePoint = SystemTime;

    fn now() -> Self::TimePoint {
        SystemTime::now()
    }
}


/// Provides waitable timer functionality.
pub struct WaitableTimer<C> {
    timer: Box<(IoContext, TimerOp)>,
    _marker: PhantomData<C>
}

impl<C: Clock> WaitableTimer<C> {
    pub fn new(ctx: &IoContext) -> Self {
        WaitableTimer {
            timer: Box::new((ctx.clone(), TimerOp::new())),
            _marker: PhantomData,
        }
    }

    // pub fn async_wait<F>(&self, handler: F) -> F::Output
    //     where F: Handler<(), io::Error>
    // {
    //     let (tx, rx) = handler.channel();
    //     self.as_ctx().do_dispatch(AsyncWait())
    //     rx.yield_return(self.as_ctx())
    // }

    pub fn cancel(&mut self) {
    }

    pub fn expires_at(&mut self, expiry: C::TimePoint) {
    }

    pub fn expires_from_now(&mut self, expiry: C::Duration) {
    }
}

unsafe impl<C> Send for WaitableTimer<C> {}

unsafe impl<C> AsIoContext for WaitableTimer<C> {
    fn as_ctx(&self) -> &IoContext {
        &self.timer.0
    }
}
