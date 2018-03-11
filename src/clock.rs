use core::{AsIoContext, Expiry, TimerImpl, IoContext, Perform, ThreadIoContext};
use ops::{Handler, async_wait, AsyncWaitOp};

use std::io;
use std::marker::PhantomData;
use std::ops::Add;
use std::time::{Duration, Instant, SystemTime};

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
    pimpl: Box<TimerImpl>,
    _marker: PhantomData<C>,
}

impl<C> WaitableTimer<C>
where
    C: Clock,
{
    pub fn new(ctx: &IoContext) -> Self {
        WaitableTimer {
            pimpl: TimerImpl::new(ctx),
            _marker: PhantomData,
        }
    }

    pub fn async_wait<F>(&self, handler: F) -> F::Output
    where
        F: Handler<(), io::Error>,
    {
        async_wait(self, handler)
    }

    pub fn cancel(&self) {
        self.pimpl.cancel()
    }

    pub fn expires_at(&self, expiry: C::TimePoint) {
        self.pimpl.reset_expiry(expiry.into());
    }

    pub fn expires_from_now(&self, expiry: C::Duration) {
        self.expires_at(C::now() + expiry);
    }

    pub fn wait(&self) -> io::Result<()> {
        Ok(())
    }
}

unsafe impl<C> AsIoContext for WaitableTimer<C> {
    fn as_ctx(&self) -> &IoContext {
        &self.pimpl.as_ctx()
    }
}

impl<C> AsyncWaitOp for WaitableTimer<C>
where
    C: Clock,
{
    fn set_wait_op(&self, this: &mut ThreadIoContext, op: Box<Perform>) {
        self.pimpl.set_wait_op(this, op)
    }
}

unsafe impl<C> Send for WaitableTimer<C> {}

unsafe impl<C> Sync for WaitableTimer<C> {}
