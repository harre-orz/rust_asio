use core::{AsIoContext, Perform, Reactor, ThreadCallStack};
use ffi::SystemError;

use std::io;
use std::mem;
use std::sync::{Arc, Condvar, Mutex};
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

    pub fn outstanding_countdown(&self) {
        self.as_ctx().0.outstanding_work.fetch_sub(1, Ordering::SeqCst);
    }
}

pub trait Exec: Send + 'static {
    fn call(self, this: &mut ThreadIoContext);

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext);
}

impl<F> Exec for F
where
    F: FnOnce(&IoContext) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        self(this.as_ctx());
        this.as_ctx()
            .0
            .outstanding_work
            .fetch_sub(1, Ordering::SeqCst);
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self(this.as_ctx());
        this.as_ctx()
            .0
            .outstanding_work
            .fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct ExecOp<F>(F);

impl<F> Perform for ExecOp<F>
where
    F: Exec,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, _: SystemError) {
        self.0.call(this)
    }
}

impl Exec for Reactor {
    fn call(self, _: &mut ThreadIoContext) {
        unreachable!();
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        println!(
            "called reactor: outstanding_work={}",
            this.as_ctx().0.outstanding_work.load(Ordering::Relaxed) + this.pending_queue.len()
        );

        if this.pending_queue.len() == 0
            && this.as_ctx().0.outstanding_work.load(Ordering::Relaxed) == 0
        {
            this.as_ctx().stop();
            println!("call stop")
        } else {
        }

        self.poll(true, this);

        if this.as_ctx().stopped() {
            // forget the reactor
            Box::into_raw(self);
            println!("forget reactor");
        } else {
            this.as_ctx().push(self);
        }
    }
}

struct Executor {
    mutex: Mutex<VecDeque<Box<Exec>>>,
    condvar: Condvar,
    stopped: AtomicBool,
    outstanding_work: AtomicUsize,
    pending_thread_count: AtomicUsize,
    reactor: *mut Reactor,
}

unsafe impl Send for Executor {}

unsafe impl Sync for Executor {}

impl Drop for Executor {
    fn drop(&mut self) {
        // release the reactor
        let _ = unsafe { Box::from_raw(self.reactor) };
        println!("release reactor");
    }
}

#[derive(Clone)]
pub struct IoContext(Arc<Executor>);

impl IoContext {
    pub fn new() -> io::Result<Self> {
        Ok(IoContext(Arc::new(Executor {
            mutex: Default::default(),
            condvar: Default::default(),
            stopped: Default::default(),
            outstanding_work: Default::default(),
            pending_thread_count: Default::default(),
            reactor: Box::into_raw(Box::new(Reactor::new()?)),
        })))
    }

    pub fn stopped(&self) -> bool {
        self.0.stopped.load(Ordering::Relaxed)
    }

    fn push(&self, exec: Box<Exec>) {
        let mut queue = self.0.mutex.lock().unwrap();
        queue.push_back(exec);
        self.0.condvar.notify_one();
    }

    pub fn restart(&self) {
        self.0.stopped.store(false, Ordering::Relaxed)
    }

    fn pop(&self) -> Option<Box<Exec>> {
        let mut queue = self.0.mutex.lock().unwrap();
        loop {
            if let Some(exec) = queue.pop_front() {
                return Some(exec);
            } else if self.stopped() {
                return None;
            }
            self.0.pending_thread_count.fetch_add(1, Ordering::Relaxed);
            queue = self.0.condvar.wait(queue).unwrap();
            self.0.pending_thread_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    #[doc(hidden)]
    pub fn do_dispatch<F>(&self, exec: F)
    where
        F: Exec,
    {
        self.0.outstanding_work.fetch_add(1, Ordering::SeqCst);
        if let Some(this) = ThreadIoContext::callstack(self) {
            exec.call(this)
        } else {
            self.push(Box::new(exec))
        }
    }

    #[doc(hidden)]
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

    #[doc(hidden)]
    pub fn do_post<F>(&self, exec: F)
    where
        F: Exec,
    {
        self.0.outstanding_work.fetch_add(1, Ordering::SeqCst);
        if let Some(this) = ThreadIoContext::callstack(self) {
            this.push_back(Box::new(ExecOp(exec)), unsafe { mem::uninitialized() })
        } else {
            self.push(Box::new(exec))
        }
    }

    pub fn dispatch<F>(&self, func: F)
    where
        F: FnOnce(&IoContext) + Send + 'static,
    {
        self.do_dispatch(func)
    }

    pub fn post<F>(&self, func: F)
    where
        F: FnOnce(&IoContext) + Send + 'static,
    {
        self.do_post(func)
    }

    pub fn run(self: &IoContext) {
        if self.stopped() {
            return;
        }

        let mut this = ThreadIoContext::new(self, Default::default());
        this.init();

        self.push(unsafe { Box::from_raw(self.0.reactor) });

        while let Some(exec) = self.pop() {
            exec.call_box(&mut this);
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

    #[doc(hidden)]
    pub fn as_reactor(&self) -> &Reactor {
        unsafe { &*self.0.reactor }
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
