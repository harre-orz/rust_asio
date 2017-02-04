use error::{ErrCode, READY};
use clock::{Clock, SteadyClock, SystemClock};
use core::{IoContext, AsIoContext, ThreadIoContext, AsyncTimer, Expiry, workplace};
use async::{Handler, Receiver, WrappedHandler, Operation};

use std::io;
use std::fmt;
use std::marker::PhantomData;

struct WaitableTimerHandler;

impl WrappedHandler<(), io::Error> for WaitableTimerHandler {
    fn perform(&mut self, ctx: &IoContext, _: &mut ThreadIoContext, ec: ErrCode, op: Operation<(), io::Error, Self>) {
        match ec {
            READY => op.send(ctx, Ok(())),
            ec => op.send(ctx, Err(ec.into())),
        }
    }
}

/// Provides waitable timer functionality.
pub struct WaitableTimer<C> {
    timer: AsyncTimer,
    _marker: PhantomData<C>,
}

impl<C: Clock> WaitableTimer<C> {
    pub fn new(ctx: &IoContext) -> WaitableTimer<C> {
        WaitableTimer {
            timer: AsyncTimer::new(ctx),
            _marker: PhantomData,
        }
    }

    pub fn async_wait<F>(&self, handler: F) -> F::Output
        where F: Handler<(), io::Error>
    {
        let (op, res) = handler.channel(WaitableTimerHandler);
        workplace(self.as_ctx(), |this| self.timer.set_timer_op(this, op.into()));
        res.recv(self.as_ctx())
    }

    pub fn cancel(&self) -> &Self {
        workplace(self.as_ctx(), |this| self.timer.set_expire_time(this, Expiry::zero()));
        self
    }

    pub fn expires_at(&self, expiry_time: C::TimePoint) -> &Self {
        workplace(self.as_ctx(), |this| self.timer.set_expire_time(this, expiry_time.into()));
        self
    }

    pub fn expires_from_now(&self, expiry_time: C::Duration) -> &Self {
        self.expires_at(C::now() + expiry_time)
    }
}

unsafe impl<C> Send for WaitableTimer<C> { }

unsafe impl<C> AsIoContext for WaitableTimer<C> {
    fn as_ctx(&self) -> &IoContext {
        self.timer.as_ctx()
    }
}

impl fmt::Debug for WaitableTimer<SteadyClock> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SteadyTimer")
    }
}

impl fmt::Debug for WaitableTimer<SystemClock> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SystemTimer")
    }
}

/// The monotonic clock's timer.
pub type SteadyTimer = WaitableTimer<SteadyClock>;

/// The system clock's timer.
pub type SystemTimer = WaitableTimer<SystemClock>;

#[test]
fn test_async_wait() {
    use async::wrap;

    use std::time::{Instant, Duration};
    use std::sync::{Arc, Mutex};

    let t1 = Instant::now();

    let ctx = &IoContext::new().unwrap();
    let t = Arc::new(Mutex::new(SteadyTimer::new(ctx)));
    t.lock().unwrap().expires_from_now(Duration::new(1, 0));
    t.lock().unwrap().async_wait(wrap(|_,_| {}, &t));
    ctx.run();

    let t2 = Instant::now();
    assert!( (t2 - t1) >= Duration::new(1,0) );
}
