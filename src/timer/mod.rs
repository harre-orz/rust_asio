use std::io;
use {IoObject, IoService, Strand};

pub trait WaitTimer : IoObject {
    type TimePoint;
    type Duration;

    fn new(io: &IoService) -> Self;

    fn wait_at(&self, time: &Self::TimePoint) -> io::Result<()>;

    fn async_wait_at<A, F, T>(a: A, time: &Self::TimePoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send;

    fn wait_for(&self, time: &Self::Duration) -> io::Result<()>;

    fn async_wait_for<A, F, T>(a: A, time: &Self::Duration, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send;

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send;
}

mod system;
pub use self::system::*;

mod steady;
pub use self::steady::*;
