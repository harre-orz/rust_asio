use core::{IoContext, AsIoContext, ThreadIoContext, Perform, Expiry, InnerTimer};
use async::{Handler, Yield, AsyncWait, AsyncWaitOp};

use std::io;
use std::marker::PhantomData;
use std::ops::Add;
use std::time::{Duration, SystemTime, Instant};

pub trait Clock: Send + 'static {
    type Duration;

    type TimePoint: Add<Self::Duration, Output = Self::TimePoint> + Into<Expiry>;

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
    inner: Box<InnerTimer>,
    _marker: PhantomData<C>,
}

impl<C> WaitableTimer<C>
where
    C: Clock,
{
    pub fn new(ctx: &IoContext) -> Self {
        WaitableTimer {
            inner: InnerTimer::new(ctx),
            _marker: PhantomData,
        }
    }

    pub fn async_wait<F>(&self, handler: F) -> F::Output
    where
        F: Handler<(), io::Error>,
    {
        let (tx, rx) = handler.channel();
        self.as_ctx().do_dispatch(AsyncWait::new(self, tx));
        rx.yield_return()
    }

    pub fn cancel(&mut self) {
        self.inner.cancel()
    }

    pub fn expires_at(&mut self, expiry: C::TimePoint) {
        self.inner.set_expiry(expiry.into())
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
        &self.inner.as_ctx()
    }
}

impl<C> AsyncWaitOp for WaitableTimer<C>
where
    C: Clock,
{
    fn set_wait_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>) {
        self.inner.set_wait_op(this, op)
    }

    fn reset_wait_op(&mut self, this: &mut ThreadIoContext) {
        self.inner.reset_wait_op(this)
    }
}
