use std::io;
use std::time;
use std::ops::{Add, Sub};
use std::marker::PhantomData;
use time::{Duration, Timespec, SteadyTime, get_time};
use {IoObject, IoService, Handler};
use io_service::{WaitActor, Expiry, ToExpiry};
use backbone::{AsWaitActor, sleep_for};
use backbone::ops::{async_wait, cancel_wait};

pub trait ToStdDuration {
    fn to_std(&self) -> time::Duration;
}

impl ToStdDuration for Duration {
    fn to_std(&self) -> time::Duration {
        self.to_std().unwrap_or(time::Duration::new(0, 0))
    }
}

pub trait Clock : Send + 'static {
    type Duration : ToStdDuration + Clone;

    type TimePoint : ToExpiry
        + Add<Self::Duration, Output = Self::TimePoint>
        + Sub<Self::TimePoint, Output = Self::Duration>;

    fn now() -> Self::TimePoint;
}

pub struct WaitTimer<C> {
    wait: WaitActor,
    marker: PhantomData<C>,
}

impl<C: Clock> WaitTimer<C> {
    pub fn new<T: IoObject>(io: &T) -> WaitTimer<C> {
        WaitTimer {
            wait: WaitActor::new(io),
            marker: PhantomData,
        }
    }

    pub fn async_wait_at<F: Handler<()>>(&self, time: C::TimePoint, handler: F) {
        async_wait(self, time.to_expiry(), handler)
    }

    pub fn async_wait_for<F: Handler<()>>(&self, time: C::Duration, handler: F) {
        async_wait(self, (C::now() + time).to_expiry(), handler)
    }

    pub fn cancel(&self) {
        cancel_wait(self);
    }

    pub fn wait_at(&self, time: C::TimePoint) -> io::Result<()> {
        sleep_for((time - C::now()).to_std())
    }

    pub fn wait_for(&self, time: C::Duration) -> io::Result<()> {
        sleep_for(time.to_std())
    }
}

impl<C> IoObject for WaitTimer<C> {
    fn io_service(&self) -> &IoService {
        self.wait.io_service()
    }
}

impl<C: Clock> AsWaitActor for WaitTimer<C> {
    fn as_wait_actor(&self) -> &WaitActor {
        &self.wait
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    type Duration = Duration;
    type TimePoint = Timespec;

    fn now() -> Timespec {
        get_time()
    }
}

pub struct SteadyClock;

impl Clock for SteadyClock {
    type Duration = Duration;
    type TimePoint = SteadyTime;

    fn now() -> SteadyTime {
        SteadyTime::now()
    }
}
