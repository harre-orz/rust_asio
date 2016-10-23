use std::io;
use std::thread;
use std::marker::PhantomData;
use std::time::Duration;
use error::{READY, CANCELED, ErrorCode, canceled, stopped};
use io_service::{IoObject, IoService, Handler, AsyncResult, TimerActor};
use clock::{Clock, SteadyClock, SystemClock, Expiry};

/// Provides waitable timer functionality.
pub struct WaitableTimer<C: Clock> {
    act: TimerActor,
    _marker: PhantomData<C>,
}

impl<C: Clock> WaitableTimer<C> {
    pub fn new(io: &IoService) -> WaitableTimer<C> {
        WaitableTimer {
            act: TimerActor::new(io),
            _marker: PhantomData,
        }
    }

    pub fn async_wait_at<F>(&self, endpoint: C::TimePoint, handler: F) -> F::Output
        where F: Handler<()>
    {
        async_wait(&self.act, C::expires_at(endpoint), handler)
    }

    pub fn async_wait_for<F>(&self, duration: C::Duration, handler: F) -> F::Output
        where F: Handler<()>
    {
        async_wait(&self.act, C::expires_from(duration), handler)
    }

    pub fn cancel(&self) {
        if let Some(callback) = self.act.unset_wait() {
            self.io_service().dispatch(move |io| callback(io, CANCELED));
        }
    }

    pub fn wait_at(&self, endpoint: C::TimePoint) -> io::Result<()> {
        sleep_for(self.io_service(), C::elapsed_at(endpoint))
    }

    pub fn wait_for(&self, duration: C::Duration) -> io::Result<()> {
        sleep_for(self.io_service(), C::elapsed_from(duration))
    }
}

unsafe impl<C: Clock> IoObject for WaitableTimer<C> {
    fn io_service(&self) -> &IoService {
        self.act.io_service()
    }
}

fn async_wait<F>(act: &TimerActor, expiry: Expiry, handler: F) -> F::Output
    where F: Handler<()>
{
    let out = handler.async_result();
    act.set_wait(expiry, Box::new(move |io: *const IoService, ec| {
        let io = unsafe { &*io };
        match ec {
            READY => handler.callback(io, Ok(())),
            CANCELED => handler.callback(io, Err(canceled())),
            ErrorCode(ec) => handler.callback(io, Err(io::Error::from_raw_os_error(ec))),
        }
    }));
    out.get(act.io_service())
}

fn sleep_for(io: &IoService, duration: Duration) -> io::Result<()> {
    if !io.stopped() {
        thread::sleep(duration);
        Ok(())
    } else {
        Err(stopped())
    }
}

/// The monotonic clock's timer.
pub type SteadyTimer = WaitableTimer<SteadyClock>;

/// The system clock's timer.
pub type SystemTimer = WaitableTimer<SystemClock>;
