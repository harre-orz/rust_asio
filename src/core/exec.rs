use ffi::SystemError;
use core::{Reactor, ThreadCallStack, TimerQueue, UnsafeRef};

use std::io;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::collections::VecDeque;

pub trait Perform: Send + 'static {
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError);
}

#[derive(Default)]
pub struct ThreadInfo {
    pending_queue: Vec<(Box<Perform>, SystemError)>,
}

pub type ThreadIoContext = ThreadCallStack<IoContext, ThreadInfo>;

impl ThreadIoContext {
    pub fn push(&mut self, op: Box<Perform>, err: SystemError) {
        self.pending_queue.push((op, err))
    }

    pub fn increase_outstanding_work(&self) {
        self.as_ctx().0.outstanding_work.fetch_add(
            1,
            Ordering::SeqCst,
        );
    }

    pub fn decrease_outstanding_work(&self) {
        self.as_ctx().0.outstanding_work.fetch_sub(
            1,
            Ordering::SeqCst,
        );
    }
}

pub trait Exec: Send + 'static {
    fn call(self, this: &mut ThreadIoContext);

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext);

    fn outstanding_work(&self, ctx: &IoContext) {
        ctx.0.outstanding_work.fetch_add(1, Ordering::SeqCst);
    }
}

impl<F> Exec for F
where
    F: FnOnce(&IoContext) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        self(this.as_ctx());
        this.decrease_outstanding_work();
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self(this.as_ctx());
        this.decrease_outstanding_work();
    }
}

impl Exec for (Box<Perform>, SystemError) {
    fn call(self, this: &mut ThreadIoContext) {
        let (op, err) = self;
        op.perform(this, err)
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }

    fn outstanding_work(&self, _: &IoContext) {}
}

struct Executor {
    mutex: Mutex<VecDeque<Box<Exec>>>,
    condvar: Condvar,
    stopped: AtomicBool,
    outstanding_work: AtomicUsize,
    timer_queue: TimerQueue,
    reactor: Reactor,
}

unsafe impl Send for Executor {}

unsafe impl Sync for Executor {}

type ExecutorRef = UnsafeRef<Executor>;

impl Exec for ExecutorRef {
    fn call(self, _: &mut ThreadIoContext) {
        unreachable!();
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        if this.as_ctx().0.outstanding_work.load(Ordering::Relaxed) == 0 {
            this.as_ctx().stop();
        } else {
            let more_handlers = this.as_ctx().0.mutex.lock().unwrap().len();
            self.reactor.poll(
                more_handlers == 0,
                &self.timer_queue,
                this,
            )
        }
        if this.as_ctx().stopped() {
            Box::into_raw(self);
        } else {
            this.as_ctx().push(self);
        }
    }

    fn outstanding_work(&self, _: &IoContext) {}
}

#[derive(Clone)]
pub struct IoContext(Arc<Executor>);

impl IoContext {
    pub fn new() -> io::Result<Self> {
        let ctx = Arc::new(Executor {
            mutex: Default::default(),
            condvar: Default::default(),
            stopped: Default::default(),
            outstanding_work: Default::default(),
            timer_queue: TimerQueue::new(),
            reactor: Reactor::new()?,
        });
        ctx.reactor.init();
        Ok(IoContext(ctx))
    }

    #[doc(hidden)]
    pub fn as_reactor(&self) -> &Reactor {
        &self.0.reactor
    }

    #[doc(hidden)]
    pub fn as_timer_queue(&self) -> &TimerQueue {
        &self.0.timer_queue
    }

    #[doc(hidden)]
    pub fn do_dispatch<F>(&self, exec: F)
    where
        F: Exec,
    {
        exec.outstanding_work(self);
        if let Some(this) = ThreadIoContext::callstack(self) {
            exec.call(this)
        } else {
            self.push(Box::new(exec))
        }
    }

    #[doc(hidden)]
    pub fn do_post<F>(&self, exec: F)
    where
        F: Exec,
    {
        exec.outstanding_work(self);
        self.push(Box::new(exec))
    }

    pub fn dispatch<F>(&self, func: F)
    where
        F: FnOnce(&IoContext) + Send + 'static,
    {
        self.do_dispatch(func)
    }

    fn pop(&self) -> Option<Box<Exec>> {
        let mut queue = self.0.mutex.lock().unwrap();
        loop {
            if let Some(exec) = queue.pop_front() {
                return Some(exec);
            } else if self.stopped() {
                return None;
            }
            queue = self.0.condvar.wait(queue).unwrap();
        }
    }

    pub fn post<F>(&self, func: F)
    where
        F: FnOnce(&IoContext) + Send + 'static,
    {
        self.do_post(func)
    }

    fn push(&self, exec: Box<Exec>) {
        let mut queue = self.0.mutex.lock().unwrap();
        queue.push_back(exec);
        self.0.condvar.notify_one();
    }

    pub fn restart(&self) {
        self.0.stopped.store(false, Ordering::Relaxed)
    }

    pub fn run(self: &IoContext) {
        if self.stopped() {
            return;
        }

        let mut this = ThreadIoContext::new(self, Default::default());
        this.init();

        self.push(Box::new(ExecutorRef::new(&*self.0)));
        while let Some(exec) = self.pop() {
            exec.call_box(&mut this);
            while !this.pending_queue.is_empty() {
                let vec: Vec<_> = this.pending_queue.drain(..).collect();
                for (op, err) in vec {
                    op.perform(&mut this, err);
                }
            }
        }
    }

    pub fn stop(&self) {
        if !self.0.stopped.swap(true, Ordering::SeqCst) {
            let _queue = self.0.mutex.lock().unwrap();
            self.as_reactor().interrupt();
            self.0.condvar.notify_all();
        }
    }

    pub fn stopped(&self) -> bool {
        self.0.stopped.load(Ordering::Relaxed)
    }
}

impl Eq for IoContext {}

impl PartialEq for IoContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

pub unsafe trait AsIoContext {
    fn as_ctx(&self) -> &IoContext;
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
        if (self.0).0.outstanding_work.fetch_sub(1, Ordering::Relaxed) == 1 {
            self.0.stop()
        }
    }
}

unsafe impl AsIoContext for IoContextWork {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.0) {
            this.as_ctx()
        } else {
            &self.0
        }
    }
}

#[test]
fn test_work() {
    let ctx = &IoContext::new().unwrap();
    {
        let _work = IoContextWork::new(ctx);
    }
    assert!(ctx.stopped());
}

#[test]
fn test_multithread_work() {
    use std::thread;
    use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

    static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

    let ctx = &IoContext::new().unwrap();
    let _work = IoContextWork::new(ctx);

    let mut thrds = Vec::new();
    for _ in 0..10 {
        let ctx = ctx.clone();
        thrds.push(thread::spawn(move || ctx.run()))
    }

    for i in 0..100 {
        ctx.post(move |ctx| if COUNT.fetch_add(1, Ordering::SeqCst) == 99 {
            ctx.stop();
        })
    }

    ctx.run();
    for thrd in thrds {
        thrd.join().unwrap();
    }

    assert_eq!(COUNT.load(Ordering::Relaxed), 100);
}
