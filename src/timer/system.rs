use std::io;
use time::{Duration, Tm, now};
use {Strand, Cancel};
use backbone::{ToExpiry, TimerActor};
use timer::WaitTimer;
use ops::*;
use ops::async::*;

pub struct SystemTimer {
    actor: TimerActor,
}

impl SystemTimer {
    pub fn new() -> Self {
        SystemTimer {
            actor: TimerActor::new(),
        }
    }
}

impl AsTimerActor for SystemTimer {
    fn as_timer_actor(&self) -> &TimerActor {
        &self.actor
    }
}

impl WaitTimer for SystemTimer {
    type TimePoint = Tm;
    type Duration = Duration;

    fn wait_at(&self, time: &Self::TimePoint) -> io::Result<()> {
        sleep_for((*time - now()).to_std())
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
        async_timer(timer, (now() + *time).to_expiry(), callback, obj)
    }
}

impl Cancel for SystemTimer {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: FnOnce(&T) -> &Self {
        cancel_timer(a(obj), obj);
    }
}
