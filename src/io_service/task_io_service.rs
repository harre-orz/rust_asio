use std::fmt;
use std::boxed::FnBox;
use std::sync::{Mutex, Condvar};
use std::sync::atomic::{Ordering, AtomicBool, AtomicUsize};
use std::collections::VecDeque;
use unsafe_cell::{UnsafeRefCell};
use error::{READY, CANCELED};
use super::{IoService, CallStack, ThreadInfo, Reactor, TimerQueue, Control};

type Callback = Box<FnBox(*const IoService) + Send + 'static>;

pub struct IoServiceImpl {
    mutex: Mutex<VecDeque<Callback>>,
    condvar: Condvar,
    stopped: AtomicBool,
    outstanding_work: AtomicUsize,
    nthreads: AtomicUsize,
    pub react: Reactor,
    pub queue: TimerQueue,
    pub ctrl: Control,
}

impl IoServiceImpl {
    pub fn new() -> IoServiceImpl {
        IoServiceImpl {
            mutex: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            stopped: AtomicBool::new(false),
            outstanding_work: AtomicUsize::new(0),
            nthreads: AtomicUsize::new(0),
            react: Reactor::new(),
            queue: TimerQueue::new(),
            ctrl: Control::new(),
        }
    }

    fn running_in_this_thread(&self) -> bool {
        CallStack::contains()
    }

    fn count(&self) -> usize {
        let task = self.mutex.lock().unwrap();
        task.len()
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        if !self.stopped.swap(true, Ordering::SeqCst) {
            let mut _task = self.mutex.lock().unwrap();
            self.ctrl.interrupt();
            self.condvar.notify_all();
        }
    }

    pub fn reset(&self) {
        self.stopped.store(false, Ordering::SeqCst);
    }

    pub fn dispatch<F>(&self, io: &IoService, func: F)
        where F: FnOnce(&IoService) + Send + 'static
    {
        if self.running_in_this_thread() {
            func(io);
        } else {
            self.post(func)
        }
    }

    pub fn post<F>(&self, func: F)
        where F: FnOnce(&IoService) + Send + 'static
    {
        let mut task = self.mutex.lock().unwrap();
        task.push_back(Box::new(move |io: *const IoService| func(unsafe { &*io })));
        self.condvar.notify_one();
    }

    fn wait(&self) -> Option<Callback> {
        let mut task = self.mutex.lock().unwrap();
        loop {
            let stoppable = self.outstanding_work.load(Ordering::Relaxed) == 0
                || self.stopped.load(Ordering::Relaxed);
            if let Some(callback) = task.pop_front() {
                return Some(callback);
            } else if stoppable {
                return None
            }
            task = self.condvar.wait(task).unwrap();
        }
    }

    fn event_loop(io: &IoService, ti: &ThreadInfo) {
        if io.stopped() {
            io.0.react.cancel_all(ti);
            io.0.queue.cancel_all(ti);
            io.0.ctrl.stop(io);
            for callback in ti.collect() {
                io.post(move |io| callback(io, CANCELED));
            }
        } else {
            let ti_ref = UnsafeRefCell::new(ti);
            io.post(move |io| {
                let ti = unsafe { ti_ref.as_ref() };
                let mut count = io.0.outstanding_work.load(Ordering::Relaxed);
                let timeout = if count > 0 && io.0.nthreads.load(Ordering::Relaxed) > 1 {
                    Some(io.0.ctrl.wait_duration(200000))
                } else {
                    None
                };
                count += io.0.react.poll(timeout, io, ti);
                count += io.0.queue.ready_expired(ti);
                count += ti.len();
                for callback in ti.collect() {
                    io.post(move |io| callback(io, READY));
                }
                if count == 0 && io.0.count() == 0 {
                    io.0.stop();
                }
                Self::event_loop(io, ti);
            });
        }
    }

    pub fn run(&self, io: &IoService) {
        if self.stopped() {
            return;
        }

        let thread_info = match ThreadInfo::new() {
            None => return,
            Some(thread_info) => thread_info,
        };

        self.nthreads.fetch_add(1, Ordering::SeqCst);
        if self.ctrl.start(io) {
            Self::event_loop(io, &thread_info);
        }
        while let Some(func) = self.wait() {
            func(io);
        }
        self.nthreads.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn work_started(&self) {
        self.outstanding_work.fetch_add(1, Ordering::SeqCst);
    }

    pub fn work_finished(&self) -> bool {
        self.outstanding_work.fetch_sub(1, Ordering::SeqCst) == 1
    }
}

impl fmt::Debug for IoServiceImpl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TaskIoService")
    }
}
