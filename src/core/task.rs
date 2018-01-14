use core::{IoContext, AsIoContext, ThreadCallStack, Reactor, Perform};
use ffi::SystemError;

use std::io;
use std::mem;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::collections::VecDeque;

#[derive(Default)]
pub struct ThreadInfo {
    pending_queue: Vec<(Box<Perform>, SystemError)>,
}

pub type ThreadIoContext = ThreadCallStack<IoContext, ThreadInfo>;


impl ThreadIoContext {
    pub fn push_back(&mut self, op: Box<Perform>, err: SystemError) {
        self.pending_queue.push((op, err))
    }

    pub fn run(&mut self) {
        let vec: Vec<_> = self.pending_queue.drain(..).collect();
        for (op, err) in vec {
            op.perform(self, err);
        }
    }
}


pub trait Task: Send + 'static {
    fn call(self, this: &mut ThreadIoContext);

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext);
}

impl<F> Task for F
where
    F: FnOnce(&IoContext) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        self(this.as_ctx());
        this.as_ctx().0.outstanding_work.fetch_sub(1, Ordering::SeqCst);
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self(this.as_ctx());
        this.as_ctx().0.outstanding_work.fetch_sub(1, Ordering::SeqCst);
    }
}


pub struct TaskOp<F>(F);

impl<F> Perform for TaskOp<F>
where
    F: Task,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, _: SystemError) {
        self.0.call(this)
    }
}


impl Task for Reactor {
    fn call(self, _: &mut ThreadIoContext) {
        unreachable!();
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        println!("called reactor: outstanding_work={}", this.as_ctx().0.outstanding_work.load(Ordering::Relaxed) + this.pending_queue.len());

        if this.pending_queue.len() == 0 && this.as_ctx().0.outstanding_work.load(Ordering::Relaxed) == 0 {
            this.as_ctx().stop();
        } else {
        }

        self.poll(true, this);

        if this.as_ctx().stopped() {
            // forget the reactor
            Box::into_raw(self);
        } else {
            this.as_ctx().push(self);
        }
    }
}


pub struct Inner {
    mutex: Mutex<VecDeque<Box<Task>>>,
    condvar: Condvar,
    stopped: AtomicBool,
    pub outstanding_work: AtomicUsize,
    pending_thread_count: AtomicUsize,
    reactor: *mut Reactor,
}

unsafe impl Send for Inner {}

unsafe impl Sync for Inner {}

impl Drop for Inner {
    fn drop(&mut self) {
        // release the reactor
        let _ = unsafe { Box::from_raw(self.reactor) };
    }
}


#[derive(Clone)]
pub struct TaskIoContext(pub Arc<Inner>);


impl TaskIoContext {
    pub fn new() -> io::Result<Self> {
        let reactor = Box::into_raw(Box::new(Reactor::new()?));
        let ctx = TaskIoContext(Arc::new(Inner {
            mutex: Default::default(),
            condvar: Default::default(),
            stopped: Default::default(),
            outstanding_work: Default::default(),
            pending_thread_count: Default::default(),
            reactor: reactor,
        }));
        ctx.push(unsafe { Box::from_raw(reactor) });
        Ok(ctx)
    }

    pub fn stopped(&self) -> bool {
        self.0.stopped.load(Ordering::Relaxed)
    }

    fn push(&self, task: Box<Task>) {
        let mut queue = self.0.mutex.lock().unwrap();
        queue.push_back(task);
        self.0.condvar.notify_one();
    }

    pub fn restart(&self) {
        self.0.stopped.store(false, Ordering::Relaxed)
    }

    fn pop(&self) -> Option<Box<Task>> {
        let mut queue = self.0.mutex.lock().unwrap();
        loop {
            if let Some(task) = queue.pop_front() {
                return Some(task);
            } else if self.stopped() {
                return None;
            }
            self.0.pending_thread_count.fetch_add(1, Ordering::Relaxed);
            queue = self.0.condvar.wait(queue).unwrap();
            self.0.pending_thread_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    pub fn do_dispatch<F>(&self, task: F)
        where F: Task,
    {
        self.0.outstanding_work.fetch_add(1, Ordering::SeqCst);
        if let Some(this) = ThreadIoContext::callstack(self) {
            task.call(this)
        } else {
            self.push(Box::new(task))
        }
    }

    pub fn do_perform(&self, op: Box<Perform>, err: SystemError) {
        self.0.outstanding_work.fetch_add(1, Ordering::SeqCst);
        if let Some(this) = ThreadIoContext::callstack(self) {
            op.perform(this, err);
        } else {
            let mut this = ThreadIoContext::new(self, Default::default());
            this.init();
            op.perform(&mut this, err);
            this.run();
        }
    }

    pub fn do_post<F>(&self, task: F)
        where F: Task,
    {
        self.0.outstanding_work.fetch_add(1, Ordering::SeqCst);
        if let Some(this) = ThreadIoContext::callstack(self) {
            this.push_back(
                Box::new(TaskOp(task)),
                unsafe { mem::uninitialized() },
            )
        } else {
            self.push(Box::new(task))
        }
    }

    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(&IoContext) + Send + 'static
    {
        self.do_dispatch(func)
    }

    pub fn post<F>(&self, func: F)
        where F: FnOnce(&IoContext) + Send + 'static
    {
        self.do_post(func)
    }

    pub fn run(self: &IoContext) {
        if self.stopped() {
            return;
        }

        let mut this = ThreadIoContext::new(self, Default::default());
        this.init();

        while let Some(task) = self.pop() {
            task.call_box(&mut this);
            this.run();
        }
    }

    pub fn stop(&self) {
        if !self.0.stopped.swap(true, Ordering::SeqCst) {
            let _queue = self.0.mutex.lock().unwrap();
            self.as_reactor().interrupt();
            self.0.condvar.notify_all();
        }
    }

    pub fn as_reactor(&self) -> &Reactor {
        unsafe { &*self.0.reactor }
    }
}


pub struct IoContextWork(IoContext);

impl IoContextWork {
    pub fn new(ctx: &IoContext) -> Self {
        (ctx.0).outstanding_work.fetch_add(1, Ordering::Relaxed);
        IoContextWork(ctx.clone())
    }
}

impl Drop for IoContextWork {
    fn drop(&mut self) {
        (self.0).0.outstanding_work.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Eq for IoContext {}

impl PartialEq for IoContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

unsafe impl AsIoContext for IoContext {
    fn as_ctx(&self) -> &IoContext {
        self
    }
}
