use std::sync::Arc;
use std::boxed::FnBox;
use error::ErrCode;
pub use std::os::unix::io::{RawFd, AsRawFd};

type Callback = Box<FnBox(*const IoService, ErrCode) + Send + 'static>;

//---------
// Reactor

#[cfg(all(feature = "epoll", target_os = "linux"))] mod epoll_reactor;
#[cfg(all(feature = "epoll", target_os = "linux"))] pub use self::epoll_reactor::{Reactor, IoActor, IntrActor};

#[cfg(all(feature = "kqueue", target_os = "macos"))] mod kqueue_reactor;
#[cfg(all(feature = "kqueue", target_os = "macos"))] pub use self::kqueue_reactor::{Reactor, IoActor, IntrActor};

#[cfg(not(any(all(feature = "epoll", target_os = "linux"), all(feature = "kqueue", target_os = "macos"))))] mod null_reactor;
#[cfg(not(any(all(feature = "epoll", target_os = "linux"), all(feature = "kqueue", target_os = "macos"))))] pub use self::null_reactor::{Reactor, IntrActor, IoActor};

//---------
// control

#[cfg(all(feature = "timerfd", target_os = "linux"))] mod timerfd_control;
#[cfg(all(feature = "timerfd", target_os = "linux"))] pub use self::timerfd_control::Control;

#[cfg(all(unix, not(all(feature = "timerfd", target_os = "linux"))))] mod pipe_control;
#[cfg(all(unix, not(all(feature = "timerfd", target_os = "linux"))))] pub use self::pipe_control::Control;

//-----------
// IoService

mod thread_info;
pub use self::thread_info::{CallStack, ThreadInfo};

mod task_io_service;
use self::task_io_service::IoServiceImpl;

mod timer_queue;
pub use self::timer_queue::{TimerQueue, TimerActor};

/// Traits to the associated with `IoService`.
pub unsafe trait IoObject : Sized {
    /// Returns a `IoService` associated with this object.
    fn io_service(&self) -> &IoService;
}

pub trait FromRawFd<P> : AsRawFd + Send + 'static {
    unsafe fn from_raw_fd(io: &IoService, pro: P, fd: RawFd) -> Self;
}

/// Provides core I/O functionality.
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
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoService;
    /// use std::sync::atomic::{Ordering, AtomicUsize, ATOMIC_USIZE_INIT};
    ///
    /// static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
    ///
    /// let io = IoService::new();
    /// io.dispatch(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// io.dispatch(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// io.dispatch(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// io.run();
    ///
    /// assert_eq!(COUNT.load(Ordering::Relaxed), 3);
    /// ```

    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(&IoService) + Send + 'static
    {
        self.0.dispatch(self, func)
    }

    /// Requests a process to invoke the given handler and return immediately.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoService;
    /// use std::sync::atomic::{Ordering, AtomicUsize, ATOMIC_USIZE_INIT};
    ///
    /// static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
    ///
    /// let io = IoService::new();
    /// io.post(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// io.post(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// io.post(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// io.run();
    ///
    /// assert_eq!(COUNT.load(Ordering::Relaxed), 3);
    /// ```
    pub fn post<F>(&self, func: F)
        where F: FnOnce(&IoService) + Send + 'static
    {
        self.0.post(func);
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

    /// Returns a `IoServiceWork` associated the `IoService`.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoService;
    ///
    /// let io = IoService::new();
    /// let mut work = Some(IoService::work(&io));
    /// assert_eq!(io.stopped(), false);
    /// work = None;
    /// assert_eq!(io.stopped(), true);
    /// ```
    pub fn work(io: &IoService) -> IoServiceWork {
        io.0.work_started();
        IoServiceWork { io: io.clone() }
    }

    pub fn strand<T, F>(io: &IoService, data: T, init: F)
        where F: FnOnce(Strand<T>)
    {
        let imp = StrandImpl::new(data, true);
        init(strand(io, &imp));
        imp.do_dispatch(io);
    }

    #[cfg(feature = "context")]
    pub fn spawn<F>(io: &IoService, func: F)
        where F: FnOnce(&Coroutine) + Send + 'static,
    {
        spawn(io, func);
    }
}

unsafe impl IoObject for IoService {
    fn io_service(&self) -> &IoService {
        self
    }
}

/// The class to delaying until the stop of `IoService` is dropped.
///
/// # Examples
/// When dropped the `IoServiceWork`, to stop the `IoService`:
///
/// ```
/// use asyncio::IoService;
/// use std::sync::atomic::{Ordering, AtomicUsize, ATOMIC_USIZE_INIT};
///
/// static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
///
/// let io = &IoService::new();
/// let mut work = Some(IoService::work(io));
///
/// fn count_if_not_stopped(io: &IoService) {
///   if !io.stopped() {
///     COUNT.fetch_add(1, Ordering::Relaxed);
///   }
/// }
/// io.post(count_if_not_stopped);
/// io.post(move |_| work = None);  // call IoService::stop()
/// io.post(count_if_not_stopped);
/// io.run();
///
/// assert_eq!(COUNT.load(Ordering::Relaxed), 1);
/// ```
///
/// # Examples
/// A multithreading example code:
///
/// ```
/// use asyncio::IoService;
/// use std::thread;
/// use std::sync::atomic::{Ordering, AtomicUsize, ATOMIC_USIZE_INIT};
///
/// static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
///
/// let io = &IoService::new();
/// let _work = IoService::work(io);
///
/// let mut thrds = Vec::new();
/// for _ in 0..10 {
///   let io = io.clone();
///   thrds.push(thread::spawn(move || io.run()));
/// }
///
/// for _ in 0..100 {
///   io.post(move |io| {
///     if COUNT.fetch_add(1, Ordering::SeqCst) == 99 {
///       io.stop();
///     }
///   });
/// }
///
/// io.run();
/// for thrd in thrds {
///   thrd.join().unwrap();
/// }
///
/// assert_eq!(COUNT.load(Ordering::Relaxed), 100);
/// ```
pub struct IoServiceWork {
    io: IoService,
}

unsafe impl IoObject for IoServiceWork {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}

impl Drop for IoServiceWork {
    fn drop(&mut self) {
        if self.io.0.work_finished() {
            self.io.stop();
        }
    }
}

mod handler;
pub use self::handler::{Handler, AsyncResult, NoAsyncResult, BoxedAsyncResult, wrap};

mod strand;
pub use self::strand::{Strand, StrandHandler, StrandImpl, strand};

#[cfg(feature = "context")] mod coroutine;
#[cfg(feature = "context")] pub use self::coroutine::{Coroutine, spawn};
