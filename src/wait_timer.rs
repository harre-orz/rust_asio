use std::io;
use std::ops::Add;
use std::marker::PhantomData;
use time::{Duration, Tm, SteadyTime, now};
use {IoObject, IoService, Strand};
use backbone::{ToExpiry, TimerActor};
use ops;
use ops::async::*;

pub trait Clock : Send + 'static {
    type Duration : Clone;
    type TimePoint : ToExpiry + Add<Self::Duration, Output = Self::TimePoint>;
    fn now() -> Self::TimePoint;
}

pub struct WaitTimer<C: Clock> {
    actor: TimerActor,
    marker: PhantomData<C>,
}

impl<C: Clock> WaitTimer<C> {
    pub fn async_wait_at<F, T>(&self, time: &C::TimePoint, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        async_wait(self, time.to_expiry(), callback, strand);
    }

    pub fn async_wait_for<F, T>(&self, time: &C::Duration, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        async_wait(self, (C::TimePoint::now() + time.clone()).to_expiry(), callback, strand);
    }

    pub fn cancel(&self) {
        cancel_wait(self)
    }

    pub fn wait_at(&self, time: &C::TimePoint) -> io::Result<()> {
        ops::sleep_for(time.to_expiry())
    }

    pub fn wait_for(&self, time: &C::Duration) -> io::Result<()> {
        ops::sleep_for((C::TimePoint::now() + time.clone()).to_expiry())
    }
}

impl<C: Clock> IoObject for WaitTimer<C> {
    fn io_service(&self) -> &IoService {
        self.actor.io_service()
    }
}

impl<C: Clock> AsTimerActor for WaitTimer<C> {
    fn as_timer_actor(&self) -> &TimerActor {
        &self.actor
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    type Duration = Duration;
    type TimePoint = Tm;

    fn now() -> Tm {
        now()
    }
}

impl WaitTimer<SystemClock> {
    pub fn new<T: IoObject>(io: &T) -> WaitTimer<SystemClock> {
        WaitTimer {
            actor: TimerActor::new(io),
            marker: PhantomData,
        }
    }
}

pub type SystemTimer = WaitTimer<SystemClock>;

pub struct SteadyClock;

impl Clock for SteadyClock {
    type Duration = Duration;
    type TimePoint = SteadyTime;

    fn now() -> SteadyTime {
        SteadyTime::now()
    }
}

impl WaitTimer<SteadyClock> {
    pub fn new<T: IoObject>(io: &T) -> WaitTimer<SteadyClock> {
        WaitTimer {
            actor: TimerActor::new(io),
            marker: PhantomData,
        }
    }
}

pub type SteadyTimer = WaitTimer<SteadyClock>;
