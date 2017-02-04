use unsafe_cell::UnsafeRefCell;
use error::ErrCode;
use core::{IoContext, AsIoContext, ThreadIoContext, FnOp, Upcast};
use async::{Sender, NullReceiver, Operation, WrappedHandler, Handler};

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

trait FnBox<T> {
    fn call_box(self: Box<Self>, ctx: &IoContext, this: &mut ThreadIoContext, data: &StrandData<T>);
}

impl<T, F: FnOnce(Strand<T>, &mut ThreadIoContext)> FnBox<T> for F {
    fn call_box(self: Box<Self>, ctx: &IoContext, this: &mut ThreadIoContext, data: &StrandData<T>) {
        (*self)(Strand { ctx: ctx, data: data }, this)
    }
}

type Function<T> = Box<FnBox<T>>;

struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<Function<T>>,
}

pub struct StrandData<T> {
    mutex: Arc<(Mutex<StrandQueue<T>>, UnsafeCell<T>)>,
}
unsafe impl<T> Send for StrandData<T> {
}

impl<T> StrandData<T> {
    pub fn run<F>(&self, ctx: &IoContext, this: &mut ThreadIoContext, func: F)
        where F: FnOnce(Strand<T>, &mut ThreadIoContext) + Send + 'static
    {
        {
            let mut owner = self.mutex.0.lock().unwrap();
            if owner.locked {
                owner.queue.push_back(Box::new(func));
                return;
            }
            owner.locked = true;
        }

        func(Strand { ctx: ctx, data: self }, this);

        while let Some(func) = {
            let mut owner = self.mutex.0.lock().unwrap();
            if let Some(func) = owner.queue.pop_front() {
                Some(func)
            } else {
                owner.locked = false;
                None
            }
        } {
            func.call_box(ctx, this, self);
        }
    }
}

impl<T> Clone for StrandData<T> {
    fn clone(&self) -> Self {
        StrandData {
            mutex: self.mutex.clone()
        }
    }
}

/// Provides serialized data and handler execution.
pub struct Strand<'a, T: 'a> {
    ctx: &'a IoContext,
    data: &'a StrandData<T>,
}

impl<'a, T> Strand<'a, T> {
    /// Returns a `&mut T` to the memory safely.
    pub fn get(&self) -> &mut T {
        unsafe { &mut *self.data.mutex.1.get() }
    }

    /// Request the strand to invoke the given handler.
    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        func(Strand { ctx: self.ctx, data: self.data })
    }

    /// Request the strand to invoke the given handler and return immediately.
    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let mut owner = self.data.mutex.0.lock().unwrap();
        owner.queue.push_back(Box::new(move|st: Strand<T>, _: &mut ThreadIoContext| func(st)))
    }

    /// Provides a `Strand` handler to asynchronous operation.
    ///
    /// The StrandHandler has trait the `Handler`, that type of `Handler::Output` is `()`.
    pub fn wrap<F, R, E>(&self, handler: F) -> StrandHandler<T, F, R, E>
        where F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
              R: Send + 'static,
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
        self.ctx
    }
}

impl<'a, T> Deref for Strand<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.data.mutex.1.get() }
    }
}

impl<'a, T> DerefMut for Strand<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.mutex.1.get() }
    }
}

/// Provides immutable data and handler execution.
pub struct StrandImmutable<'a, T> {
    ctx: &'a IoContext,
    data: StrandData<T>,
}

impl<'a, T: 'static> StrandImmutable<'a, T> {
    /// Request the strand to invoke the given handler.
    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let data = self.data.clone();
        self.ctx.do_dispatch(move|ctx: &IoContext, this: &mut ThreadIoContext| {
            data.run(ctx, this, move|st: Strand<T>, _: &mut ThreadIoContext| {
                func(st)
            })
        })
    }

    /// Request the strand to invoke the given handler and return immediately.
    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let data = self.data.clone();
        self.ctx.do_post(move|ctx: &IoContext, this: &mut ThreadIoContext| {
            data.run(ctx, this, move|st: Strand<T>, _: &mut ThreadIoContext| {
                func(st)
            })
        })
    }

    pub unsafe fn as_mut(&'a self) -> Strand<'a, T> {
        Strand {
            ctx: self.ctx,
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
        unsafe { &*self.data.mutex.1.get() }
    }
}

/// The binding Strand handler.
pub struct StrandHandler<T, F, R, E> {
    pub data: StrandData<T>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}

impl<T, F, R, E> StrandHandler<T, F, R, E>
    where T: 'static,
          F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
{
    pub fn send(self, ctx: &IoContext, res: Result<R, E>) {
        let StrandHandler { data, handler, _marker } = self;
        handler(Strand { ctx: ctx, data: &data }, res)
    }
}

impl<T, F, R, E> Handler<R, E> for StrandHandler<T, F, R, E>
    where T: 'static,
          F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
{
    type Output = ();

    type Receiver = NullReceiver;

    fn channel<G>(self, op: G) -> (Operation<R, E, G>, Self::Receiver)
        where G: WrappedHandler<R, E> + Send + 'static
    {
        (Box::new((self, op)), NullReceiver)
    }

    fn result(self, ctx: &IoContext, res: Result<R, E>) -> Self::Output {
        let StrandHandler { data, handler, _marker } = self;
        handler(Strand { ctx: ctx, data: &data }, res)
    }
}

impl<T, F, R, E, G> Upcast<FnOp + Send> for (StrandHandler<T, F, R, E>, G)
    where T: 'static,
          F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
          G: WrappedHandler<R, E> + Send + 'static,
{
    fn upcast(self: Box<Self>) -> Box<FnOp + Send> {
        self
    }
}

impl<T, F, R, E, G> Sender<R, E, G> for (StrandHandler<T, F, R, E>, G)
    where T: 'static,
          F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
          G: WrappedHandler<R, E> + Send + 'static,
{
    fn send(self: Box<Self>, ctx: &IoContext, res: Result<R, E>) {
        ctx.post(move|ctx| self.0.send(ctx, res))
    }

    fn as_self(&self) -> &G {
        &self.1
    }

    fn as_mut_self(&mut self) -> &mut G {
        &mut self.1
    }
}

impl<T, F, R, E, G> FnOp for (StrandHandler<T, F, R, E>, G)
    where T: 'static,
          F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
          G: WrappedHandler<R, E> + Send + 'static,
{
    fn call_op(self: Box<Self>, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode) {
        self.0.data.clone().run(ctx, this, move |st, this| {
            let mut g = UnsafeRefCell::new(&self.1);
            unsafe { g.as_mut() }.perform(st.as_ctx(), this, ec, self)
        })
    }
}

pub fn strand_clone<'a, T>(ctx: &'a IoContext, data: &'a StrandData<T>) -> Strand<'a, T> {
    Strand { ctx: ctx, data: data }
}

impl IoContext {
    pub fn strand<'a, T>(ctx: &'a IoContext, data: T) -> StrandImmutable<'a, T> {
        StrandImmutable {
            ctx: ctx,
            data: StrandData {
                mutex: Arc::new((Mutex::new(StrandQueue {
                    locked: false,
                    queue: VecDeque::new(),
                }), UnsafeCell::new(data)))
            },
        }
    }
}

#[test]
fn test_strand() {
    let ctx = &IoContext::new().unwrap();
    let st = IoContext::strand(ctx, 0);
    let mut st = unsafe { st.as_mut() };
    *st = 1;
    assert_eq!(*st, 1);
}

#[test]
fn test_strand_dispatch() {
    let ctx = &IoContext::new().unwrap();
    let st = IoContext::strand(ctx, 0);
    st.dispatch(|mut st| *st = 1);
    ctx.run();
    assert_eq!(*st, 1);
}

#[test]
fn test_strand_post() {
    let ctx = &IoContext::new().unwrap();
    let st = IoContext::strand(ctx, 0);
    st.post(|mut st| *st = 1);
    ctx.run();
    assert_eq!(*st, 1);
}
