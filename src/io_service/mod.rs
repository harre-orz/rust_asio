use std::sync::Arc;
use std::boxed::FnBox;
use error::ErrorCode;
pub use std::os::unix::io::{RawFd, AsRawFd};

type Callback = Box<FnBox(*const IoService, ErrorCode) + Send + 'static>;

//---------
// Reactor

#[cfg(all(not(feature = "asyncio_no_epoll"), target_os = "linux"))]
mod epoll_reactor;
#[cfg(all(not(feature = "asyncio_no_epoll"), target_os = "linux"))]
pub use self::epoll_reactor::{Reactor, IoActor, IntrActor};

// mod null_reactor;
// pub use self::null_reactor::{Reactor, IntrActor, IoActor};


//---------
// Control

#[cfg(target_os = "linux")]
mod timerfd_control;
#[cfg(target_os = "linux")]
pub use self::timerfd_control::Control;

#[cfg(all(unix, not(target_os = "linux")))]
mod pipe_control;
#[cfg(all(unix, not(target_os = "linux")))]
pub use self::pipe_control::Control;

//-----------
// IoService

mod thread_info;
pub use self::thread_info::{CallStack, ThreadInfo};

mod task_io_service;
use self::task_io_service::IoServiceImpl;

mod timer_queue;
pub use self::timer_queue::{TimerQueue, TimerActor};

/// Traits to the associated with `IoService`.
pub trait IoObject : Sized {
    /// Returns a `IoService` associated with this object.
    fn io_service(&self) -> &IoService;
}

pub trait FromRawFd<P> : AsRawFd + Send + 'static {
    unsafe fn from_raw_fd(io: &IoService, pro: P, fd: RawFd) -> Self;
}

#[derive(Clone, Debug)]
pub struct IoService(Arc<IoServiceImpl>);

impl IoService {
    /// Returns a new `IoService`.
    ///
    /// # Panics
    /// Panics if too many open files.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoService;
    ///
    /// let io = IoService::new();
    /// ```
    pub fn new() -> IoService {
        IoService(Arc::new(IoServiceImpl::new()))
    }

    /// Requests a process to invoke the given handler.
    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(&IoService) + Send + 'static
    {
        self.0.dispatch(self, func)
    }

    /// Requests a process to invoke the given handler and return immediately.
    pub fn post<F>(&self, func: F)
        where F: FnOnce(&IoService) + Send + 'static
    {
        self.0.post(func);
    }

    /// Runs all given handlers.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoService;
    ///
    /// let io = IoService::new();
    /// io.run();
    /// ```
    pub fn run(&self) {
        self.0.run(self)
    }

    /// Sets a stop request and cancel all of the waiting event in an `IoService`.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoService;
    ///
    /// let io = IoService::new();
    /// io.stop();
    /// ```
    pub fn stop(&self) {
        self.0.stop()
    }

    /// Returns true if this has been stopped.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoService;
    ///
    /// let io = IoService::new();
    /// assert_eq!(io.stopped(), false);
    /// io.stop();
    /// assert_eq!(io.stopped(), true);
    /// ```
    pub fn stopped(&self) -> bool {
        self.0.stopped()
    }

    /// Resets a stopped `IoService`.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoService;
    ///
    /// let io = IoService::new();
    /// assert_eq!(io.stopped(), false);
    /// io.stop();
    /// assert_eq!(io.stopped(), true);
    /// io.reset();
    /// assert_eq!(io.stopped(), false);
    /// ```
    pub fn reset(&self) {
        self.0.reset()
    }

    fn work_started(&self) {
        self.0.work_started()
    }

    fn work_finished(&self) {
        if self.0.work_finished() {
            self.stop()
        }
    }

    pub fn work(io: &IoService) -> IoServiceWork {
        io.work_started();
        IoServiceWork(io.clone())
    }
}

impl IoObject for IoService {
    fn io_service(&self) -> &IoService {
        self
    }
}

pub struct IoServiceWork(IoService);

impl IoObject for IoServiceWork {
    fn io_service(&self) -> &IoService {
        &self.0
    }
}

impl Drop for IoServiceWork {
    fn drop(&mut self) {
        self.0.work_finished()
    }
}

mod handler;
pub use self::handler::{Handler, AsyncResult, NoAsyncResult, Strand, wrap};
#[cfg(feature = "context")] pub use self::handler::{Coroutine, spawn};
