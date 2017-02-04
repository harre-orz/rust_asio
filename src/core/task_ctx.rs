use unsafe_cell::UnsafeBoxedCell;
use error::ErrCode;
use core::{IoContext, Init, ThreadCallStack, Operation, Reactor, Scheduler, Interrupter};

use std::io;
use std::fmt;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::collections::VecDeque;

trait FnBox {
    fn call_box(self: Box<Self>, ctx: &IoContext, this: &mut ThreadIoContext) -> usize;
}

impl<F: FnOnce(&IoContext, &mut ThreadIoContext)> FnBox for F {
    fn call_box(self: Box<Self>, ctx: &IoContext, this: &mut ThreadIoContext) -> usize
    {
        (*self)(ctx, this);
        1
    }
}

type Function = Box<FnBox + Send + 'static>;

pub struct TaskIoContext {
    pub reactor: UnsafeBoxedCell<Reactor>,
    pub scheduler: Scheduler,
    pub interrupter: Interrupter,
    outstanding_work: Arc<AtomicUsize>,

    mutex: Mutex<VecDeque<Function>>,
    condvar: Condvar,
    stopped: AtomicBool,
    running: AtomicBool,
    registry: Init,
}

impl Drop for TaskIoContext {
    fn drop(&mut self) {
        self.interrupter.cleanup(&*self.reactor);
        self.scheduler.cleanup(&*self.reactor);
        self.reactor.release();
    }
}

impl TaskIoContext {
    pub fn new() -> io::Result<IoContext> {
        let registry = Init::registry();
        let outstanding_work = Arc::new(AtomicUsize::default());

        let reactor = UnsafeBoxedCell::new(try!(Reactor::new(outstanding_work.clone())));
        let scheduler = try!(Scheduler::new(outstanding_work.clone()));
        scheduler.startup(&*reactor);

        let interrupter = try!(Interrupter::new());
        interrupter.startup(&*reactor);

        Ok(IoContext(Arc::new(TaskIoContext {
            reactor: reactor,
            scheduler: scheduler,
            interrupter: interrupter,
            outstanding_work: outstanding_work,

            mutex: Default::default(),
            condvar: Default::default(),
            stopped: Default::default(),
            running: Default::default(),

            registry: registry,
        })))
    }

    pub fn do_dispatch<F>(ctx: &IoContext, func: F)
        where F: FnOnce(&IoContext, &mut ThreadIoContext) + Send + 'static
    {
        if let Some(this) = ThreadCallStack::contains(ctx) {
            func(ctx, this);
        } else {
            ctx.0.push(Box::new(func));
        }
    }

    pub fn do_post<F>(ctx: &IoContext, func: F)
        where F: FnOnce(&IoContext, &mut ThreadIoContext) + Send + 'static
    {
        ctx.0.push(Box::new(func))
    }

    pub fn restart(ctx: &IoContext) -> bool {
        if let Some(_) = ThreadCallStack::contains(ctx) {
            false
        } else {
            ctx.0.stopped.swap(false, Ordering::SeqCst)
        }
    }

    pub fn run(ctx: &IoContext) -> usize {
        if ctx.stopped() {
            return 0;
        }

        if !ctx.0.running.swap(true, Ordering::SeqCst) {
            ctx.0.push(ctx.0.reactor.release());
        }

        let mut cs = ThreadCallStack::new(Default::default());
        let mut this_thread = cs.wind(ctx);

        let mut n = 0;
        while let Some(func) = ctx.0.wait() {
            n += func.call_box(ctx, &mut this_thread);
        }

        n
    }

    pub fn run_one(ctx: &IoContext) -> usize {
        if ctx.stopped() {
            return 0;
        }

        if ctx.0.running.swap(true, Ordering::SeqCst) == false {
            ctx.0.push(ctx.0.reactor.release());
        }

        let mut cs = ThreadCallStack::new(Default::default());
        let mut this_thread = cs.wind(ctx);

        while let Some(func) = ctx.0.wait() {
            if func.call_box(ctx, &mut *this_thread) > 0 {
                return 1;
            }
        }

        0
    }

    pub fn stop(ctx: &IoContext) {
        workplace(ctx, |this| {
            ctx.0.stop_all_threads();
            ctx.0.reactor.cancel_all_fds(this);
            ctx.0.scheduler.cancel_all_timers(this);
        })
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    pub fn work_started(&self) {
        self.outstanding_work.fetch_add(1, Ordering::SeqCst);
    }

    pub fn work_finished(&self) {
        self.outstanding_work.fetch_sub(1, Ordering::SeqCst);
    }

    fn push(&self, func: Function) {
        let mut task = self.mutex.lock().unwrap();
        self.outstanding_work.fetch_add(1, Ordering::SeqCst);
        task.push_back(func);
        self.condvar.notify_one();
    }

    fn stop_all_threads(&self) {
        if !self.stopped.swap(true, Ordering::SeqCst) {
            let mut _task = self.mutex.lock().unwrap();
            self.interrupter.interrupt();
            self.condvar.notify_all();
        }
    }

    fn wait(&self) -> Option<Function> {
        let mut task = self.mutex.lock().unwrap();
        loop {
            if let Some(func) = task.pop_front() {
                self.outstanding_work.fetch_sub(1, Ordering::SeqCst);
                return Some(func)
            } else if self.stopped() {
                return None;
            }
            task = self.condvar.wait(task).unwrap();
        }
    }
}

impl fmt::Debug for TaskIoContext {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

impl FnBox for Reactor {
    fn call_box(self: Box<Self>, ctx: &IoContext, this: &mut ThreadIoContext) -> usize {
        if ctx.0.outstanding_work.load(Ordering::Relaxed) == 0 {
            ctx.0.stop_all_threads();
        } else if this.len() == 0 {
            let more_handlers = ctx.0.mutex.lock().unwrap().len();
            self.run(&ctx.0.scheduler, more_handlers == 0, this);
        }
        this.run(ctx);

        if ctx.0.stopped() {
            ctx.0.running.store(false, Ordering::SeqCst);
            Box::into_raw(self);  // forget the reactor
        } else {
            ctx.0.push(self);  // repeat after last task
        }
        0
    }
}

#[derive(Default)]
pub struct ThreadIoContext {
    private_work_queue: Vec<(Operation, ErrCode)>,
}

impl ThreadIoContext {
    pub fn len(&self) -> usize {
        self.private_work_queue.len()
    }

    pub fn push(&mut self, op: Operation, ec: ErrCode) {
        self.private_work_queue.push((op, ec))
    }

    pub fn run(&mut self, ctx: &IoContext) {
        for (op, ec) in self.private_work_queue.drain(..) {
            ctx.do_post(move|ctx: &IoContext, this: &mut ThreadIoContext| {
                op.call_op(ctx, this, ec)
            })
        }
    }
}

pub fn workplace<F>(ctx: &IoContext, func: F)
    where F: FnOnce(&mut ThreadIoContext),
{
    if let Some(this) = ThreadCallStack::contains(ctx) {
        func(this)
    } else {
        let mut this = ThreadIoContext::default();
        func(&mut this);
        this.run(ctx);
    }
}
