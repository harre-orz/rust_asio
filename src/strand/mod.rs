use core::{AsIoContext, Exec, IoContext, ThreadIoContext};
use handler::{Handler, Complete};

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::ops::{Deref, DerefMut};

trait StrandExec<T>: Send + 'static {
    fn call(self, this: &mut ThreadIoContext, data: &Arc<StrandImpl<T>>);

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext, data: &Arc<StrandImpl<T>>);
}

impl<T, F> StrandExec<T> for F
where
    F: FnOnce(Strand<T>) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext, data: &Arc<StrandImpl<T>>) {
        self(Strand {
            this: this,
            data: data,
        });
        this.decrease_outstanding_work();
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext, data: &Arc<StrandImpl<T>>) {
        self(Strand {
            this: this,
            data: data,
        });
        this.decrease_outstanding_work();
    }
}

struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<Box<StrandExec<T>>>,
}

struct StrandImpl<T> {
    pub mutex: Mutex<StrandQueue<T>>,
    pub cell: UnsafeCell<T>,
}

impl<T> StrandImpl<T> {
    fn run<F>(this: &mut ThreadIoContext, data: &Arc<StrandImpl<T>>, exec: F)
    where
        T: 'static,
        F: StrandExec<T>,
    {
        {
            let mut owner = data.mutex.lock().unwrap();
            if owner.locked {
                owner.queue.push_back(Box::new(exec));
                return;
            }
            owner.locked = true;
        }

        exec.call(this, data);

        while let Some(exec) = {
            let mut owner = data.mutex.lock().unwrap();
            if let Some(exec) = owner.queue.pop_front() {
                Some(exec)
            } else {
                owner.locked = false;
                None
            }
        }
        {
            exec.call_box(this, data);
        }
    }
}

unsafe impl<T> Send for StrandImpl<T> {}

unsafe impl<T> Sync for StrandImpl<T> {}


pub struct StrandHandler<T, F, R, E> {
    data: Arc<StrandImpl<T>>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}

impl<T, F, R, E> Handler<R, E> for StrandHandler<T, F, R, E>
where
    T: 'static,
    F: FnOnce(Strand<T>, Result<R, E>)
        + Send
        + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    type Output = ();

    #[doc(hidden)]
    type Handler = Self;

    #[doc(hidden)]
    fn wrap<W>(self, ctx: &IoContext, wrapper: W) -> Self::Output
        where W: FnOnce(&IoContext, Self::Handler)
    {
        wrapper(ctx, self)
    }
}

impl<T, F, R, E> Complete<R, E> for StrandHandler<T, F, R, E>
where
    T: 'static,
    F: FnOnce(Strand<T>, Result<R, E>)
        + Send
        + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    fn success(self, this: &mut ThreadIoContext, res: R) {
        let StrandHandler {
            data,
            handler,
            _marker,
        } = self;
        StrandImpl::run(this, &data, |strand: Strand<T>| handler(strand, Ok(res)))
    }

    fn failure(self, this: &mut ThreadIoContext, err: E) {
        let StrandHandler {
            data,
            handler,
            _marker,
        } = self;
        StrandImpl::run(this, &data, |strand: Strand<T>| handler(strand, Err(err)))
    }
}

/// Provides serialized data and handler execution.
pub struct Strand<'a, T: 'a> {
    this: &'a mut ThreadIoContext,
    data: &'a Arc<StrandImpl<T>>,
}

impl<'a, T> Strand<'a, T>
where
    T: 'static,
{
    pub fn new(ctx: &'a IoContext, data: T) -> StrandImmutable<'a, T> {
        StrandImmutable {
            ctx: ctx,
            data: Arc::new(StrandImpl {
                mutex: Mutex::new(StrandQueue {
                    locked: false,
                    queue: VecDeque::new(),
                }),
                cell: UnsafeCell::new(data),
            }),
        }
    }

    /// Returns a `&mut T` to the memory safely.
    pub fn get(&self) -> &mut T {
        unsafe { &mut *self.data.cell.get() }
    }

    /// Request the strand to invoke the given handler.
    pub fn dispatch<F>(&self, func: F)
    where
        F: FnOnce(Strand<T>) + Send + 'static,
    {
        self.this.increase_outstanding_work();
        let this = &mut unsafe { &mut *(self as *const _ as *mut Self) }.this;
        StrandImpl::run(this, self.data, func)
    }

    /// Request the strand to invoke the given handler and return immediately.
    pub fn post<F>(&self, func: F)
    where
        F: FnOnce(Strand<T>) + Send + 'static,
    {
        self.this.increase_outstanding_work();
        let mut owner = self.data.mutex.lock().unwrap();
        debug_assert_eq!(owner.locked, true);
        owner.queue.push_back(Box::new(func))
    }

    /// Provides a `Strand` handler to asynchronous operation.
    ///
    /// The StrandHandler has trait the `Handler`, that type of `Handler::Output` is `()`.
    pub fn wrap<F, R, E>(&self, handler: F) -> StrandHandler<T, F, R, E>
    where
        F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        StrandHandler {
            data: self.data.clone(),
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<'a, T> AsIoContext for Strand<'a, T> {
    fn as_ctx(&self) -> &IoContext {
        self.this.as_ctx()
    }
}

impl<'a, T> Deref for Strand<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data.cell.get() }
    }
}

impl<'a, T> DerefMut for Strand<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data.cell.get() }
    }
}

impl<T, F> Exec for (Arc<StrandImpl<T>>, F)
where
    T: 'static,
    F: FnOnce(Strand<T>) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        let (data, func) = self;
        StrandImpl::run(this, &data, func)
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

/// Provides immutable data and handler execution.
pub struct StrandImmutable<'a, T> {
    ctx: &'a IoContext,
    data: Arc<StrandImpl<T>>,
}

impl<'a, T> StrandImmutable<'a, T>
where
    T: 'static,
{
    /// Request the strand to invoke the given handler.
    pub fn dispatch<F>(&self, func: F)
    where
        F: FnOnce(Strand<T>) + Send + 'static,
    {
        let data = self.data.clone();
        self.ctx.do_dispatch((data, func))
    }

    /// Request the strand to invoke the given handler and return immediately.
    pub fn post<F>(&self, func: F)
    where
        F: FnOnce(Strand<T>) + Send + 'static,
    {
        let data = self.data.clone();
        self.ctx.do_post((data, func))
    }

    pub unsafe fn get(&mut self) -> &mut T {
        &mut *(self.data.cell.get())
    }

    #[doc(hidden)]
    pub fn make_mut(&'a self, this: &'a mut ThreadIoContext) -> Strand<'a, T> {
        Strand {
            this: this,
            data: &self.data,
        }
    }
}

unsafe impl<'a, T> AsIoContext for StrandImmutable<'a, T> {
    fn as_ctx(&self) -> &IoContext {
        self.ctx
    }
}

impl<'a, T> Deref for StrandImmutable<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data.cell.get() }
    }
}


#[cfg(feature = "context")]
mod coroutine;
#[cfg(feature = "context")]
pub use self::coroutine::{spawn, Coroutine, CoroutineHandler};

#[test]
fn test_strand() {
    let ctx = &IoContext::new().unwrap();
    let st = Strand::new(ctx, 0);
    assert_eq!(*st, 0);
}

#[test]
fn test_strand_dispatch() {
    let ctx = &IoContext::new().unwrap();
    let st = Strand::new(ctx, 0);
    st.dispatch(|mut st| *st = 1);
    ctx.run();
    assert_eq!(*st, 1);
}

#[test]
fn test_strand_post() {
    let ctx = &IoContext::new().unwrap();
    let st = Strand::new(ctx, 0);
    st.post(|mut st| *st = 1);
    ctx.run();
    assert_eq!(*st, 1);
}
