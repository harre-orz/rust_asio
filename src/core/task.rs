use super::{IoContext, AsIoContext, ThreadCallStack, Reactor};

use std::io;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::collections::VecDeque;

#[derive(Default)]
pub struct ThreadInfo {
    working_count: usize,
    private_queue: Vec<Box<Task>>,
}

pub type ThreadIoContext = ThreadCallStack<IoContext, ThreadInfo>;


impl ThreadIoContext {
    pub fn push<F: Task>(&mut self, task: F) {
        if self.as_ctx().0.pending_thread_count.load(Ordering::Relaxed) > 0 {
            self.as_ctx().0.push(task);
        } else {
            self.private_queue.push(box task);
        }
    }

    pub fn run(&mut self) {
    }
}


pub trait Task : Send + 'static {
    fn call(self, this: &mut ThreadIoContext);

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext);
}


struct TaskOnce<F>(F);

impl<F> Task for TaskOnce<F>
    where F: FnOnce(&IoContext) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        self.0(this.as_ctx());
        this.working_count += 1;
        this.as_ctx().0.outstanding_work_count.fetch_sub(1, Ordering::Relaxed);
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext)  {
        self.call(this)
    }
}

impl Task for Reactor {
    fn call(self, this: &mut ThreadIoContext) {
        self.poll(true, this);

        if !this.as_ctx().stopped() {
            this.as_ctx().do_dispatch(self);
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}


pub struct TaskIoContext {
    mutex: Mutex<VecDeque<Box<Task>>>,
    condvar: Condvar,
    stopped: AtomicBool,
    outstanding_work_count: AtomicUsize,
    pending_thread_count: AtomicUsize,
    pub reactor: Reactor,
}

impl TaskIoContext {
    pub fn new() -> io::Result<IoContext> {
        Ok(IoContext(Arc::new(TaskIoContext {
            mutex: Default::default(),
            condvar: Default::default(),
            stopped: Default::default(),
            outstanding_work_count: Default::default(),
            pending_thread_count: Default::default(),
            reactor: Reactor::new()?,
        })))
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    fn push<F: Task>(&self, task: F) {
        let mut queue = self.mutex.lock().unwrap();
        queue.push_back(box task);
        self.condvar.notify_one();
    }

    pub fn restart(&self) {
        self.stopped.store(false, Ordering::Relaxed)
    }

    fn pop(&self) -> Option<Box<Task>> {
        let mut queue = self.mutex.lock().unwrap();
        loop {
            if let Some(task) = queue.pop_front() {
                return Some(task)
            } else if self.stopped() {
                return None
            }
            self.pending_thread_count.fetch_add(1, Ordering::Relaxed);
            queue = self.condvar.wait(queue).unwrap();
            self.pending_thread_count.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

impl TaskIoContext {
    pub fn do_dispatch<F: Task>(ctx: &IoContext, task: F) {
        if let Some(this) = ThreadIoContext::callstack(ctx) {
            task.call(this)
        } else {
            ctx.0.outstanding_work_count.fetch_add(1, Ordering::Relaxed);
            ctx.0.push(task)
        }
    }

    pub fn do_post<F: Task>(ctx: &IoContext, task: F) {
        ctx.0.outstanding_work_count.fetch_add(1, Ordering::Relaxed);
        if let Some(this) = ThreadIoContext::callstack(ctx) {
            this.push(task)
        } else {
            ctx.0.push(task)
        }
    }

    pub fn dispatch<F>(ctx: &IoContext, func: F)
        where F: FnOnce(&IoContext) + Send + 'static
    {
        Self::do_dispatch(ctx, TaskOnce(func))
    }

    pub fn post<F>(ctx: &IoContext, func: F)
        where F: FnOnce(&IoContext) + Send + 'static
    {
        Self::do_post(ctx, TaskOnce(func))
    }

    pub fn run(ctx: &IoContext) -> usize {
        if ctx.0.stopped() {
            return 0;
        }

        let mut this = ThreadIoContext::new(ctx, Default::default());
        this.init();

        while let Some(task) = ctx.0.pop() {
            task.call_box(&mut this);
            let vec: Vec<_> = this.private_queue.drain(..).collect();
            for task in vec {
                task.call_box(&mut this);
            }
        }
        this.working_count
    }

    pub fn run_one(ctx: &IoContext) -> usize {
        // TODO
        Self::run(ctx)
    }

    pub fn stop(ctx: &IoContext) {
        if !ctx.0.stopped.swap(true, Ordering::Relaxed) {
        }
    }
}


pub struct IoContextWork(IoContext);

impl IoContextWork {
    pub fn new(ctx: &IoContext) -> Self {
        ctx.0.outstanding_work_count.fetch_add(1, Ordering::Relaxed);
        IoContextWork(ctx.clone())
    }
}

impl Drop for IoContextWork {
    fn drop(&mut self) {
        (self.0).0.outstanding_work_count.fetch_sub(1, Ordering::Relaxed);
    }
}
