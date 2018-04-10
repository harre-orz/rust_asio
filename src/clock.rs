use ffi::{SystemError};
use core::{AsIoContext, Exec, Expiry, TimerImpl, IoContext, Perform, ThreadIoContext};
use handler::{Complete, Handler};

use std::io;
use std::marker::PhantomData;
use std::ops::Add;
use std::time::{Duration, Instant, SystemTime};

pub trait AsyncWaitOp: AsIoContext + Send + 'static {
    fn set_wait_op(&self, this: &mut ThreadIoContext, op: Box<Perform>);
}

struct AsyncWait<W, F> {
    wait: *const W,
    handler: F,
}

unsafe impl<W, F> Send for AsyncWait<W, F> {}

impl<W, F> Complete<(), io::Error> for AsyncWait<W, F>
where
    W: AsyncWaitOp,
    F: Complete<(), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: ()) {
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        self.handler.failure(this, err)
    }
}

impl<W, F> Perform for AsyncWait<W, F>
where
    W: AsyncWaitOp,
    F: Complete<(), io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        if err == SystemError::default() {
            self.success(this, ())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<W, F> Exec for AsyncWait<W, F>
where
    W: AsyncWaitOp,
    F: Complete<(), io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let wait = unsafe { &*self.wait };
        wait.set_wait_op(this, Box::new(self))
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let wait = unsafe { &*self.wait };
        wait.set_wait_op(this, self)
    }
}

fn async_wait<W, F>(wait: &W, handler: F) -> F::Output
where
    W: AsyncWaitOp,
    F: Handler<(), io::Error>,
{
    handler.wrap(wait.as_ctx(), |ctx, handler| {
        ctx.do_dispatch(AsyncWait {
            wait: wait,
            handler: handler,
        })
    })
}

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
