use std::sync::Arc;
use std::marker::PhantomData;

#[cfg(all(not(feature = "asyncio_no_epoll"), target_os = "linux"))]
mod epoll_reactor;

#[cfg(all(not(feature = "asyncio_no_epoll"), target_os = "linux"))]
pub use self::epoll_reactor::{Reactor, IoActor, IntrActor};

#[cfg(all(not(feature = "asyncio_no_timerfd"), target_os = "linux"))]
mod timerfd_control;

#[cfg(all(not(feature = "asyncio_no_timerfd"), target_os = "linux"))]
pub use self::timerfd_control::Control;

mod timer_queue;
pub use self::timer_queue::{Expiry, ToExpiry, TimerQueue, WaitActor};

mod task_io_service;
use self::task_io_service::IoServiceImpl;

/// Traits to the associated with `IoService`.
pub trait IoObject : Sized {
    /// Returns a `IoService` associated with this object.
    fn io_service(&self) -> &IoService;
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
        IoService(Arc::new(IoServiceImpl::new().unwrap()))
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
        self.0.post(self, func);
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

    /// Runs all given handlers until call the `stop()`.
    ///
    /// This is ensured to not exit until explicity stopped, so it can invoking given handlers in multi-threads.
    ///
    /// # Examples
    /// Execute 5 parallels event loop (4 thread::spawn + 1 main thread).
    ///
    /// ```
    /// use asyncio::IoService;
    /// use std::thread;
    ///
    /// let mut thrds = Vec::new();
    /// let io = &IoService::new();
    /// let _ = {
    ///     let _work = IoService::work(io);
    ///     for _ in 0..4 {
    ///         let io = io.clone();
    ///         thrds.push(thread::spawn(move || io.run()));
    ///     }
    ///
    ///     io.post(move |io| {
    ///         io.stop();  // If does not explicity stop, not returns in this `work()`.
    ///     });
    ///     io.run();
    /// };
    ///
    /// for thrd in thrds {
    ///     thrd.join().unwrap();
    /// }
    /// ```
    pub fn work(io: &IoService) -> IoServiceWork {
        io.work_started();
        IoServiceWork(io.clone())
    }

    pub fn spawn<F>(io: &IoService, func: F)
        where F: FnOnce(&Coroutine) + 'static
    {
        spawn(io, func)
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

/// The binding Strand handler.
pub struct StrandHandler<T, F, R> {
    owner: StrandImpl<T>,
    handler: F,
    marker: PhantomData<R>,
}

pub struct Strand<'a, T> {
    io: &'a IoService,
    owner: StrandImpl<T>,
}

mod wrap;
pub use self::wrap::{ArcHandler, wrap};

mod strand;
pub use self::strand::{StrandImpl};

mod coroutine;
pub use self::coroutine::{CoroutineHandler, Coroutine, spawn};
