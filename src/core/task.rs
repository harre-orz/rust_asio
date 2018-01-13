use core::{IoContext, AsIoContext, ThreadCallStack, Reactor, Perform, Intr, TimerQueue};
use ffi::SystemError;

use std::io;
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


pub trait Task : Send + 'static {
    fn call(self, this: &mut ThreadIoContext);

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext);
}

impl<F> Task for F
    where F: FnOnce(&IoContext) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        self(this.as_ctx())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self(this.as_ctx())
    }
}


pub struct TaskOp<F>(F);

impl<F> Perform for TaskOp<F>
    where F: Task,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, _: SystemError) {
        self.0.call(this)
    }
}


impl Task for Reactor {
    fn call(self, this: &mut ThreadIoContext) {
        self.poll(true, this);
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.poll(true, this);
    }
}


pub struct TaskIoContext {
    mutex: Mutex<VecDeque<Box<Task>>>,
    condvar: Condvar,
    stopped: AtomicBool,
    outstanding_work_count: AtomicUsize,
    pending_thread_count: AtomicUsize,
    pub reactor: Reactor,
    pub intr: Intr,
    pub tq: TimerQueue,
}

impl TaskIoContext {
    pub fn new() -> io::Result<IoContext> {
        let ctx = TaskIoContext {
            mutex: Default::default(),
            condvar: Default::default(),
            stopped: Default::default(),
            outstanding_work_count: Default::default(),
            pending_thread_count: Default::default(),
            reactor: Reactor::new()?,
            intr: Intr::new()?,
            tq: TimerQueue::new()?,
        };
        ctx.intr.startup(&ctx.reactor);
        ctx.tq.startup(&ctx.reactor);
        Ok(IoContext(Arc::new(ctx)))
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    fn push(&self, task: Box<Task>) {
        let mut queue = self.mutex.lock().unwrap();
        queue.push_back(task);
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

impl Drop for TaskIoContext {
    fn drop(&mut self) {
        self.tq.cleanup(&self.reactor);
        self.intr.cleanup(&self.reactor);
    }
}

impl TaskIoContext {
    pub fn do_dispatch<F: Task>(ctx: &IoContext, task: F) {
        if let Some(this) = ThreadIoContext::callstack(ctx) {
            task.call(this)
        } else {
            ctx.0.outstanding_work_count.fetch_add(1, Ordering::Relaxed);
            ctx.0.push(Box::new(task))
        }
    }

    pub fn do_perform(ctx: &IoContext, op: Box<Perform>, err: SystemError) {
        if let Some(this) = ThreadIoContext::callstack(ctx) {
            op.perform(this, err);
        } else {
            let mut this = ThreadIoContext::new(ctx, Default::default());
            this.init();
            op.perform(&mut this, err);
            this.run();
        }
    }

    pub fn do_post<F: Task>(ctx: &IoContext, task: F) {
        ctx.0.outstanding_work_count.fetch_add(1, Ordering::Relaxed);
        if let Some(this) = ThreadIoContext::callstack(ctx) {
            this.push_back(Box::new(TaskOp(task)), unsafe { ::std::mem::uninitialized() })
        } else {
            ctx.0.push(Box::new(task))
        }
    }

    pub fn run(ctx: &IoContext) -> usize {
        if ctx.0.stopped() {
            return 0;
        }

        let mut this = ThreadIoContext::new(ctx, Default::default());
        this.init();

        while let Some(task) = ctx.0.pop() {
            task.call_box(&mut this);
            this.run();
        }
        0
    }

    pub fn run_one(ctx: &IoContext) -> usize {
        // TODO
        Self::run(ctx)
    }

    pub fn stop(ctx: &IoContext) {
        if !ctx.0.stopped.swap(true, Ordering::Relaxed) {
            ctx.0.intr.interrupt();
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
