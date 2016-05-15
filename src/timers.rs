use super::*;
use std::io;

pub trait Timer<'a> : IoObject<'a> {
    type TimePoint;
    type Duration;
    fn now() -> Self::TimePoint;
    fn wait_at(&self, time: &Self::TimePoint) -> io::Result<()>;
    fn wait_for(&self, time: &Self::Duration) -> io::Result<()>;
}

pub mod timer {
    use super::*;
    use super::super::{IoService, IoObject, BasicTimer};
    use std::io;
    use time::*;
    use libc;

    pub struct SystemTimer<'a> {
        _impl: BasicTimer<'a>,
    }

    impl<'a> IoObject<'a> for SystemTimer<'a> {
        fn io_service(&self) -> &'a IoService {
            self._impl.io
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
            self._impl.wait(time)
        }
    }

    pub struct SteadyTimer<'a> {
        _impl: BasicTimer<'a>,
    }

    impl<'a> IoObject<'a> for SteadyTimer<'a> {
        fn io_service(&self) -> &'a IoService {
            self._impl.io
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
            self._impl.wait(time)
        }
    }
}
