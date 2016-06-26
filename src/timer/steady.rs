use std::io;
use time::{Duration, SteadyTime};
use {IoObject, IoService, Strand, Cancel};
use backbone::{ToExpiry, TimerActor};
use timer::WaitTimer;
use ops::*;
use ops::async::*;

pub struct SteadyTimer {
    actor: TimerActor,
}

impl SteadyTimer {
    pub fn new(io: &IoService) -> Self {
        SteadyTimer {
            actor: TimerActor::new(io),
        }
    }
}

impl IoObject for SteadyTimer {
    fn io_service(&self) -> &IoService {
        self.actor.io_service()
    }
}

impl AsTimerActor for SteadyTimer {
    fn as_timer_actor(&self) -> &TimerActor {
        &self.actor
    }
}

impl WaitTimer for SteadyTimer {
    type TimePoint = SteadyTime;
    type Duration = Duration;

    fn wait_at(&self, time: &Self::TimePoint) -> io::Result<()> {
        sleep_for((*time - SteadyTime::now()).to_std())
    }

    fn async_wait_at<A, F, T>(a: A, time: &Self::TimePoint, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        let timer = a(obj);
        async_timer(timer, time.to_expiry(), callback, obj)
    }

    fn wait_for(&self, time: &Self::Duration) -> io::Result<()> {
        sleep_for(time.to_std())
    }

    fn async_wait_for<A, F, T>(a: A, time: &Self::Duration, callback: F, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self + Send + 'static,
              F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        let timer = a(obj);
        async_timer(timer, (SteadyTime::now() + *time).to_expiry(), callback, obj)
    }
}

impl Cancel for SteadyTimer {
    fn cancel(&self) {
        cancel_timer(self)
    }
}
