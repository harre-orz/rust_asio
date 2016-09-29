use std::io;
use std::fmt;
use std::sync::Arc;
use std::boxed::FnBox;
use std::collections::VecDeque;
use std::sync::{Mutex, Condvar};
use std::sync::atomic::{Ordering, AtomicBool, AtomicUsize};
use backbone::{Control, Reactor, TimerQueue};
use {IoObject, IoService};

type TaskHandler = Box<FnBox(*const IoService) + Send + 'static>;

struct TaskQueue {
    queue: VecDeque<TaskHandler>,
}

pub struct TaskExecutor {
    mutex: Mutex<TaskQueue>,
    condvar: Condvar,
    stopped: AtomicBool,
    outstanding_work: AtomicUsize,
}

impl TaskExecutor {
    fn new() -> TaskExecutor {
        TaskExecutor {
            mutex: Mutex::new(TaskQueue {
                queue: VecDeque::new(),
            }),
            condvar: Condvar::new(),
            stopped: AtomicBool::new(false),
            outstanding_work: AtomicUsize::new(0),
        }
    }

    fn count(&self) -> usize {
        let task = self.mutex.lock().unwrap();
        task.queue.len()
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        if !self.stopped.swap(true, Ordering::SeqCst) {
            self.condvar.notify_all();
        }
    }

    pub fn reset(&self) {
        self.stopped.store(false, Ordering::SeqCst)
    }

    fn post(&self, handler: TaskHandler) {
        let mut task = self.mutex.lock().unwrap();
        task.queue.push_back(handler);
        self.condvar.notify_one();
    }

    fn pop(&self) -> Option<TaskHandler> {
        let mut task = self.mutex.lock().unwrap();
        loop {
            let is_stop = self.outstanding_work.load(Ordering::Relaxed) == 0|| self.stopped.load(Ordering::Relaxed);
            if let Some(handler) = task.queue.pop_front() {
                return Some(handler);
            } else if is_stop {
                return None
            }
            task = self.condvar.wait(task).unwrap();
        }
    }
}

pub struct IoServiceBase {
    pub task: TaskExecutor,
    pub ctrl: Control,
    pub react: Reactor,
    pub queue: TimerQueue,
}

impl IoServiceBase {
    pub fn new() -> io::Result<IoServiceBase> {
        Ok(IoServiceBase {
            task: TaskExecutor::new(),
            ctrl: try!(Control::new()),
            react: try!(Reactor::new()),
            queue: TimerQueue::new(),
        })
    }

    pub fn stop(io: &IoService) {
        io.0.task.stop();
        io.0.ctrl.stop_interrupt();
    }

    pub fn post<F>(&self, handler: F)
        where F: FnOnce(&IoService) + Send + 'static {
        self.task.post(Box::new(move |io: *const IoService| handler(unsafe { &*io })));
    }

    fn dispatch(io: &IoService) {
        if io.stopped() {
            io.0.react.cancel_all(io);
            io.0.queue.cancel_all(io);
            io.0.ctrl.stop_polling(io);
        } else {
            io.post(move |io| {
                let mut count = io.0.task.outstanding_work.load(Ordering::Relaxed);
                count += io.0.react.poll(count > 0, &io);
                count += io.0.queue.cancel_expired(&io);
                if count == 0 && io.0.task.count() == 0 {
                    io.0.task.stop();
                }
                Self::dispatch(&io);
            });
        }
    }

    pub fn run(io: &IoService) {
        if io.0.ctrl.start_polling(io) {
            Self::dispatch(io);
        }
        while let Some(handler) = io.0.task.pop() {
            handler(io);
        }
    }

    pub fn work_started(&self) {
        self.task.outstanding_work.fetch_add(1, Ordering::SeqCst);
    }

    pub fn work_finished(&self) -> bool {
        self.task.outstanding_work.fetch_sub(1, Ordering::SeqCst) == 1
    }
}

impl IoService {
    /// Constructs a new `IoService`.
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
        IoService(Arc::new(IoServiceBase::new().unwrap()))
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
        IoServiceBase::stop(self)
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
        self.0.task.stopped()
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
        self.0.task.reset()
    }

    /// Requests a process to invoke the given handler and return immediately.
    ///
    /// # Examples
    /// ```
    /// use asyncio::IoService;
    /// use std::sync::atomic::*;
    ///
    /// let io = IoService::new();
    /// static PASS: AtomicBool = ATOMIC_BOOL_INIT;
    ///
    /// io.post(|_| PASS.store(true, Ordering::Relaxed));
    /// assert_eq!(PASS.load(Ordering::Relaxed), false);
    ///
    /// io.run();
    /// assert_eq!(PASS.load(Ordering::Relaxed), true);
    /// ```
    pub fn post<F>(&self, handler: F)
        where F: FnOnce(&IoService) + Send + 'static
    {
        self.0.post(handler);
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
        if !self.stopped() {
            IoServiceBase::run(self)
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
        io.0.work_started();
        IoServiceWork { io: io.clone() }
    }
}

impl IoObject for IoService {
    fn io_service(&self) -> &IoService {
        self
    }
}

impl fmt::Debug for IoService {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IoService")
    }
}

pub struct IoServiceWork {
    io: IoService,
}

impl Drop for IoServiceWork {
    fn drop(&mut self) {
        if self.io.0.work_finished() {
            self.io.stop()
        }
    }
}
