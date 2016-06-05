use std::io;
use time::{Duration, SteadyTime};
use {IoObject, IoService, Strand};
use backbone::{ToExpiry, TimerActor};
use timer::WaitTimer;
use ops::*;
use ops::async::*;

pub struct SteadyTimer {
    actor: TimerActor,
}

impl IoObject for SteadyTimer {
    fn io_service(&self) -> IoService {
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

    fn new(io: &IoService) -> Self {
        SteadyTimer {
            actor: TimerActor::register(io),
        }
    }

    fn wait_at(&self, time: &Self::TimePoint) -> io::Result<()> {
        sleep_for((*time - SteadyTime::now()).to_std())
    }

    fn async_wait_at<A, F, T>(a: A, time: &Self::TimePoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        async_timer(a, time.to_expiry(), callback, obj)
    }

    fn wait_for(&self, time: &Self::Duration) -> io::Result<()> {
        sleep_for(time.to_std())
    }

    fn async_wait_for<A, F, T>(a: A, time: &Self::Duration, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        async_timer(a, (SteadyTime::now() + *time).to_expiry(), callback, obj)
    }

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static {
        cancel_timer(a, obj);
    }
}

impl Drop for SteadyTimer {
    fn drop(&mut self) {
        self.actor.unregister();
    }
}
