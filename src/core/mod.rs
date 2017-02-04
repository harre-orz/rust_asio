use prelude::Protocol;
use ffi::{RawFd, AsRawFd};
use error::ErrCode;

use std::io;
use std::sync::Arc;

pub unsafe trait AsIoContext {
    fn as_ctx(&self) -> &IoContext;
}

#[derive(Clone, Debug)]
pub struct IoContext(Arc<Impl>);

impl IoContext {
    /// Returns a new `IoContext`.
    ///
    /// # Panics
    /// Panics if too many open files.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoContext;
    ///
    /// IoContext::new().unwrap();
    /// ```
    pub fn new() -> io::Result<IoContext> {
        Impl::new()
    }

    /// Requests a process to invoke the given handler.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoContext;
    /// use std::sync::atomic::{Ordering, AtomicUsize, ATOMIC_USIZE_INIT};
    ///
    /// static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
    ///
    /// let ctx = IoContext::new().unwrap();
    /// ctx.dispatch(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// ctx.dispatch(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// ctx.dispatch(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// assert_eq!(COUNT.load(Ordering::Relaxed), 0);
    ///
    /// ctx.run();
    /// assert_eq!(COUNT.load(Ordering::Relaxed), 3);
    /// ```
    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(&IoContext) + Send + 'static
    {
        self.do_dispatch(move|ctx: &IoContext, _: &mut ThreadIoContext| func(ctx))
    }

    #[doc(hidden)]
    pub fn do_dispatch<F>(&self, func: F)
        where F: FnOnce(&IoContext, &mut ThreadIoContext) + Send + 'static
    {
        Impl::do_dispatch(self, func)
    }

    /// Requests a process to invoke the given handler and return immediately.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoContext;
    /// use std::sync::atomic::{Ordering, AtomicUsize, ATOMIC_USIZE_INIT};
    ///
    /// static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
    ///
    /// let ctx = IoContext::new().unwrap();
    /// ctx.post(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// ctx.post(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// ctx.post(|_| { COUNT.fetch_add(1, Ordering::SeqCst); });
    /// assert_eq!(COUNT.load(Ordering::Relaxed), 0);
    ///
    /// ctx.run();
    /// assert_eq!(COUNT.load(Ordering::Relaxed), 3);
    /// ```
    pub fn post<F>(&self, func: F)
        where F: FnOnce(&IoContext) + Send + 'static
    {
        self.do_post(move|ctx: &IoContext, _: &mut ThreadIoContext| func(ctx))
    }

    #[doc(hidden)]
    pub fn do_post<F>(&self, func: F)
        where F: FnOnce(&IoContext, &mut ThreadIoContext) + Send + 'static
    {
        Impl::do_post(self, func)
    }

    /// Resets a stopped `IoContext`.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoContext;
    ///
    /// let ctx = IoContext::new().unwrap();
    /// assert_eq!(ctx.stopped(), false);
    /// ctx.stop();
    /// assert_eq!(ctx.stopped(), true);
    /// ctx.restart();
    /// assert_eq!(ctx.stopped(), false);
    /// ```
    pub fn restart(&self) -> bool {
        Impl::restart(self)
    }

    /// Runs all given handlers.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoContext;
    ///
    /// let ctx = IoContext::new().unwrap();
    /// ctx.run();
    /// ```
    pub fn run(&self) -> usize {
        Impl::run(self)
    }

    pub fn run_one(&self) -> usize {
        Impl::run_one(self)
    }

    /// Sets a stop request and cancel all of the waiting event in an `IoContext`.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoContext;
    ///
    /// let ctx = IoContext::new().unwrap();
    /// ctx.stop();
    /// ```
    pub fn stop(&self) {
        Impl::stop(self)
    }

    /// Returns true if this has been stopped.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoContext;
    ///
    /// let ctx = IoContext::new().unwrap();
    /// assert_eq!(ctx.stopped(), false);
    /// ctx.stop();
    /// assert_eq!(ctx.stopped(), true);
    /// ```
    pub fn stopped(&self) -> bool {
        self.0.stopped()
    }

    pub fn work(ctx: &IoContext) -> IoContextWork {
        ctx.0.work_started();
        IoContextWork(ctx.clone())
    }
}

unsafe impl AsIoContext for IoContext {
    fn as_ctx(&self) -> &IoContext {
        self
    }
}

unsafe impl Send for IoContext { }

/// The class to delaying until the stop of `IoContext` is dropped.
///
/// # Examples
/// When dropped the `IoContextWork`, to stop the `IoContext`:
///
/// ```
/// use asyncio::IoContext;
/// use std::sync::atomic::{Ordering, AtomicUsize, ATOMIC_USIZE_INIT};
///
/// static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
///
/// let ctx = &IoContext::new().unwrap();
/// let mut work = Some(IoContext::work(ctx));
///
/// fn count_if_not_stopped(ctx: &IoContext) {
///   if !ctx.stopped() {
///     COUNT.fetch_add(1, Ordering::Relaxed);
///   }
/// }
/// ctx.post(count_if_not_stopped);
/// ctx.post(move |_| work = None);  // call IoContext::stop()
/// ctx.post(count_if_not_stopped);
/// ctx.run();
///
/// assert_eq!(COUNT.load(Ordering::Relaxed), 1);
/// ```
///
/// # Examples
/// A multithreading example code:
///
/// ```rust,no_run
/// use asyncio::IoContext;
/// use std::thread;
/// use std::sync::atomic::{Ordering, AtomicUsize, ATOMIC_USIZE_INIT};
///
/// static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
///
/// let ctx = &IoContext::new().unwrap();
/// let _work = IoContext::work(ctx);
///
/// let mut thrds = Vec::new();
/// for _ in 0..10 {
///   let ctx = ctx.clone();
///   thrds.push(thread::spawn(move|| ctx.run()));
/// }
///
/// for _ in 0..100 {
///   ctx.post(move|ctx| {
///     if COUNT.fetch_add(1, Ordering::SeqCst) == 99 {
///       ctx.stop();
///     }
///   });
/// }
///
/// ctx.run();
/// for thrd in thrds {
///   thrd.join().unwrap();
/// }
///
/// assert_eq!(COUNT.load(Ordering::Relaxed), 100);
/// ```
pub struct IoContextWork(IoContext);

impl Drop for IoContextWork {
    fn drop(&mut self) {
        (self.0).0.work_finished();
        self.0.stop();
    }
}

unsafe impl AsIoContext for IoContextWork {
    fn as_ctx(&self) -> &IoContext {
        &self.0
    }
}

pub trait Socket<P: Protocol> : AsIoContext + AsRawFd + Send + 'static {
    unsafe fn from_raw_fd(&IoContext, pro: P, fd: RawFd) -> Self;
    fn protocol(&self) -> P;
}

pub trait Upcast<T: ?Sized>  {
    fn upcast(self: Box<Self>) -> Box<T>;
}

pub trait FnOp {
    fn call_op(self: Box<Self>, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode);
}

type Operation = Box<FnOp + Send>;

mod task_ctx;
pub use self::task_ctx::{TaskIoContext as Impl, ThreadIoContext, workplace};

mod init;
pub use self::init::Init;

mod callstack;
pub use self::callstack::ThreadCallStack;

mod reactor;
pub use self::reactor::*;

mod interrupter;
pub use self::interrupter::*;

mod scheduler;
pub use self::scheduler::*;

#[test]
fn test_new() {
    IoContext::new().unwrap();
}

#[test]
fn test_run() {
    let ctx = &IoContext::new().unwrap();
    ctx.run();
    assert!(ctx.stopped());
}

#[test]
fn test_run_one() {
    let ctx = &IoContext::new().unwrap();
    ctx.run_one();
    assert!(ctx.stopped());
}

#[test]
fn test_work() {
    let ctx = &IoContext::new().unwrap();
    {
        let _work = IoContext::work(ctx);
    }
    assert!(ctx.stopped());
}

#[test]
fn test_multithread_working() {
    use std::thread;
    use std::sync::atomic::{Ordering, AtomicUsize, ATOMIC_USIZE_INIT};

    static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

    let ctx = &IoContext::new().unwrap();
    let _work = IoContext::work(ctx);

    let mut thrds = Vec::new();
    for _ in 0..10 {
        let ctx = ctx.clone();
        thrds.push(thread::spawn(move|| ctx.run()))
    }

    for _ in 0..100 {
        ctx.post(move |ctx| {
            if COUNT.fetch_add(1, Ordering::SeqCst) == 99 {
                ctx.stop();
            }
        })
    }

    ctx.run();
    for thrd in thrds {
        thrd.join().unwrap();
    }

    assert_eq!(COUNT.load(Ordering::Relaxed), 100);
}
