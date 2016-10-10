use std::io;
use std::fmt;
use std::boxed::FnBox;
use std::sync::{Mutex, Condvar};
use std::sync::atomic::{Ordering, AtomicBool, AtomicUsize};
use std::collections::VecDeque;
use super::{IoService, Reactor, TimerQueue, Control};

type Callback = Box<FnBox(*const IoService) + Send + 'static>;

pub struct IoServiceImpl {
    mutex: Mutex<VecDeque<Callback>>,
    condvar: Condvar,
    stopped: AtomicBool,
    outstanding_work: AtomicUsize,
    pub react: Reactor,
    pub queue: TimerQueue,
    pub ctrl: Control,
}

impl IoServiceImpl {
    pub fn new() -> io::Result<IoServiceImpl> {
        Ok(IoServiceImpl {
            mutex: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            stopped: AtomicBool::new(false),
            outstanding_work: AtomicUsize::new(0),
            react: try!(Reactor::new()),
            queue: TimerQueue::new(),
            ctrl: try!(Control::new()),
        })
    }

    pub fn count(&self) -> usize {
        let task = self.mutex.lock().unwrap();
        task.len()
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        if !self.stopped.swap(true, Ordering::SeqCst) {
            self.ctrl.stop_interrupt();
            self.condvar.notify_all();
        }
    }

    pub fn reset(&self) {
        self.stopped.store(false, Ordering::SeqCst);
    }

    pub fn dispatch<F>(&self, io: &IoService, func: F)
        where F: FnOnce(&IoService) + Send + 'static
    {
        let mut task = self.mutex.lock().unwrap();
        task.push_back(Box::new(move |io: *const IoService| func(unsafe { &*io })));
        self.condvar.notify_one();
    }

    pub fn post<F>(&self, io: &IoService, func: F)
        where F: FnOnce(&IoService) + Send + 'static
    {
        let mut task = self.mutex.lock().unwrap();
        task.push_back(Box::new(move |io: *const IoService| func(unsafe { &*io })));
        self.condvar.notify_one();
    }

    fn wait(&self) -> Option<Callback> {
        let mut task = self.mutex.lock().unwrap();
        loop {
            let is_stopped = self.outstanding_work.load(Ordering::Relaxed) == 0
                || self.stopped.load(Ordering::Relaxed);
            if let Some(callback) = task.pop_front() {
                return Some(callback);
            } else if is_stopped {
                return None
            }
            task = self.condvar.wait(task).unwrap();
        }
    }

    fn event_loop(io: &IoService) {
        io.post(move |io| {
            let mut count = io.0.outstanding_work.load(Ordering::Relaxed);
            count += io.0.react.poll(count > 0, io);
            count += io.0.queue.cancel_expired(io);
            if count == 0 && io.0.count() == 0 {
                io.0.stop();
            }
            if io.stopped() {
                io.0.react.cancel_all(io);
                io.0.queue.cancel_all(io);
                io.0.ctrl.stop_polling(io);
            } else {
                Self::event_loop(io);
            }
        });
    }

    pub fn run(&self, io: &IoService) {
        if self.ctrl.start_polling(io) {
            Self::event_loop(io);
        }
        while let Some(callback) = self.wait() {
            callback(io)
        }
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
