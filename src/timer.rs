use super::*;
use super::{BasicTimer};
use std::io;
use std::mem;
use time::*;

trait Epoch {
    fn zero() -> Self;
}

impl Epoch for Tm {
    fn zero() -> Self {
        empty_tm()
    }
}

impl Epoch for SteadyTime {
    fn zero() -> Self {
        unsafe { mem::zeroed() }
    }
}

pub struct SystemTimer<'a> {
    io: &'a IoService,
    _impl: Box<BasicTimer>,
}

impl<'a> SystemTimer<'a> {
    pub fn new(io: &'a IoService) -> io::Result<SystemTimer<'a>> {
        let timer = SystemTimer {
            io: io,
            _impl: BasicTimer::default(),
        };
        io.register_timer(&timer._impl);
        Ok(timer)
    }
}

impl<'a> Drop for SystemTimer<'a> {
    fn drop(&mut self) {
        self.io.unregister_timer(&self._impl);
    }
}

impl<'a> IoObject<'a> for SystemTimer<'a> {
    fn io_service(&self) -> &'a IoService {
        self.io
    }
}

impl<'a> Timer<'a> for SystemTimer<'a> {
    type TimePoint = Tm;
    type Duration = Duration;

    fn now() -> Self::TimePoint {
        now()
    }

    fn wait_at(&self, time: &Self::TimePoint) -> io::Result<()> {
        self.wait_for(&(*time - Self::now()))
    }

    fn wait_for(&self, time: &Self::Duration) -> io::Result<()> {
        self._impl.wait(time.to_std().unwrap())
    }

    fn async_wait_at<F: FnOnce(io::Result<()>) + Send + 'static>(&mut self, time: &Self::TimePoint, func: F) {
        self._impl.async_wait(self.io, (*time - Self::TimePoint::zero()).to_std().unwrap(), func);
    }

    fn async_wait_for<F: FnOnce(io::Result<()>) + Send + 'static>(&mut self, time: &Self::Duration, func: F) {
        self.async_wait_at(&(Self::now() + *time), func)
    }

    fn cancel(&mut self) {
        self._impl.cancel(self.io);
    }
}

pub struct SteadyTimer<'a> {
    io: &'a IoService,
    _impl: Box<BasicTimer>,
}

impl<'a> SteadyTimer<'a> {
    pub fn new(io: &'a IoService) -> io::Result<SteadyTimer<'a>> {
        let timer = SteadyTimer {
            io: io,
            _impl: BasicTimer::default(),
        };
        io.register_timer(&timer._impl);
        Ok(timer)
    }
}

impl<'a> Drop for SteadyTimer<'a> {
    fn drop(&mut self) {
        self.io.unregister_timer(&self._impl);
    }
}

impl<'a> IoObject<'a> for SteadyTimer<'a> {
    fn io_service(&self) -> &'a IoService {
        self.io
    }
}

impl<'a> Timer<'a> for SteadyTimer<'a> {
    type TimePoint = SteadyTime;
    type Duration = Duration;

    fn now() -> Self::TimePoint {
        SteadyTime::now()
    }

    fn wait_at(&self, time: &Self::TimePoint) -> io::Result<()> {
        self.wait_for(&(*time - Self::now()))
    }

    fn wait_for(&self, time: &Self::Duration) -> io::Result<()> {
        self._impl.wait(time.to_std().unwrap())
    }

    fn async_wait_at<F: FnOnce(io::Result<()>) + Send + 'static>(&mut self, time: &Self::TimePoint, func: F) {
        self._impl.async_wait(self.io, (*time - Self::TimePoint::zero()).to_std().unwrap(), func);
    }

    fn async_wait_for<F: FnOnce(io::Result<()>) + Send + 'static>(&mut self, time: &Self::Duration, func: F) {
        self.async_wait_at(&(Self::now() + *time), func)
    }

    fn cancel(&mut self) {
        self._impl.cancel(self.io);
    }
}

// #[test]
// fn test_timer() {
//     use time;
//     let io = IoService::new().unwrap();
//     let mut timer = SteadyTimer::new(&io).unwrap();
//     let _ = timer.async_wait_for(&time::Duration::seconds(5), move |res| println!("{:?}", res));
//     io.task();
//     io.run();
//     assert!(false);
// }
