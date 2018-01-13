use ffi::SystemError;
use core::{IoContext, AsIoContext, ThreadIoContext, Task, Perform, AsyncWaitOp, Expiry, TimerOp};
use async::{Handler, Yield, AsyncWait};

use std::io;
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

impl<C> WaitableTimer<C>
    where C: Clock
{
    pub fn new(ctx: &IoContext) -> Self {
        WaitableTimer {
            timer: Box::new((ctx.clone(), TimerOp::new())),
            _marker: PhantomData,
        }
    }

    pub fn async_wait<F>(&self, handler: F) -> F::Output
        where F: Handler<(), io::Error>
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncWait::new(self, tx));
        rx.yield_return()
    }

    pub fn cancel(&mut self) {
        let &mut (ref ctx, ref mut timer) = &mut *self.timer;
        timer.cancel_timer_op(ctx)
    }

    pub fn expires_at(&mut self, expiry: C::TimePoint) {
        let &mut (ref ctx, ref mut timer) = &mut *self.timer;
        timer.cancel_timer_op(ctx);
        timer.expiry = expiry.into();
    }

    pub fn expires_from_now(&mut self, expiry: C::Duration) {
        self.expires_at(C::now() + expiry)
    }

    pub fn wait(&self) -> io::Result<()> {
        Ok(())
    }
}

unsafe impl<C> Send for WaitableTimer<C> {}

unsafe impl<C> AsIoContext for WaitableTimer<C> {
    fn as_ctx(&self) -> &IoContext {
        &self.timer.0
    }
}


impl<C> AsyncWaitOp for WaitableTimer<C>
    where C: Clock
{
    fn add_wait_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.timer.1.set_timer_op(this, op)
    }

    fn next_wait_op(&mut self, this: &mut ThreadIoContext) {
    }
}
