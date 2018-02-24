use ffi::SystemError;
use core::{AsIoContext, Exec, IoContext, Perform, ThreadIoContext};

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::ops::{Deref, DerefMut};


pub trait AsyncReadOp: AsIoContext + Send + 'static {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn next_read_op(&self, this: &mut ThreadIoContext);
}

pub trait AsyncWriteOp: AsIoContext + Send + 'static {
    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn next_write_op(&self, this: &mut ThreadIoContext);
}

pub trait AsyncWaitOp: AsIoContext + Send + 'static {
    fn set_wait_op(&self, this: &mut ThreadIoContext, op: Box<Perform>);
}

pub struct Failure<T, F, R, E>(T, F, PhantomData<(R, E)>);

impl<T, F, R, E> Failure<T, F, R, E> {
    pub fn new(err: T, handler: F) -> Self {
        Failure(err, handler, PhantomData)
    }
}

impl<T, F, R, E> Exec for Failure<T, F, R, E>
where
    T: Into<E> + Send + 'static,
    F: Complete<R, E>,
    R: Send + 'static,
    E: Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        let Failure(err, handler, _marker) = self;
        handler.failure(this, err.into())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

pub trait Handler<R, E>: Send + 'static {
    type Output;

    #[doc(hidden)]
    type Caller: Complete<R, E>;

    #[doc(hidden)]
    type Callee: Yield<Self::Output>;

    #[doc(hidden)]
    fn channel(self) -> (Self::Caller, Self::Callee);
}

pub trait Complete<R, E>: Handler<R, E> {
    fn success(self, this: &mut ThreadIoContext, res: R);

    fn failure(self, this: &mut ThreadIoContext, err: E);
}

pub trait Yield<T> {
    fn yield_return(self) -> T;
}

pub struct NoYield;

impl Yield<()> for NoYield {
    fn yield_return(self) {}
}

pub struct ArcHandler<T, F, R, E> {
    data: Arc<T>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}

impl<T, F, R, E> Handler<R, E> for ArcHandler<T, F, R, E>
where
    T: AsIoContext + Send + Sync + 'static,
    F: FnOnce(Arc<T>, Result<R, E>)
        + Send
        + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    type Output = ();

    #[doc(hidden)]
    type Caller = Self;

    #[doc(hidden)]
    type Callee = NoYield;

    #[doc(hidden)]
    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<T, F, R, E> Complete<R, E> for ArcHandler<T, F, R, E>
where
    T: AsIoContext + Send + Sync + 'static,
    F: FnOnce(Arc<T>, Result<R, E>)
        + Send
        + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    fn success(self, this: &mut ThreadIoContext, res: R) {
        let ArcHandler {
            data,
            handler,
            _marker,
        } = self;
        handler(data, Ok(res));
        this.decrease_outstanding_work();
    }

    fn failure(self, this: &mut ThreadIoContext, err: E) {
        let ArcHandler {
            data,
            handler,
            _marker,
        } = self;
        handler(data, Err(err));
        this.decrease_outstanding_work();
    }
}

/// Provides a `Arc` handler to asynchronous operation.
///
/// The ArcHandler has trait the `Handler`, that type of `Handler::Output` is `()`.
///
/// # Examples
///
/// ```
/// use std::io;
/// use std::sync::{Arc, Mutex};
/// use asyncio::{IoContext, wrap};
/// use asyncio::ip::{IpProtocol, Tcp, TcpSocket, TcpEndpoint, TcpListener};
///
/// fn on_accept(soc: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
///   if let Ok((acc, ep)) = res {
///     println!("accepted {}", ep)
///   }
/// }
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = Arc::new(TcpListener::new(ctx, Tcp::v4()).unwrap());
/// soc.async_accept(wrap(on_accept, &soc));
/// ```
pub fn wrap<T, F, R, E>(handler: F, data: &Arc<T>) -> ArcHandler<T, F, R, E> {
    ArcHandler {
        data: data.clone(),
        handler: handler,
        _marker: PhantomData,
    }
}

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
    mutex: Mutex<StrandQueue<T>>,
    cell: UnsafeCell<T>,
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
    type Caller = Self;

    #[doc(hidden)]
    type Callee = NoYield;

    #[doc(hidden)]
    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
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
pub use self::coroutine::*;

mod accept_ops;
pub use self::accept_ops::*;

mod connect_ops;
pub use self::connect_ops::*;

mod read_ops;
pub use self::read_ops::*;

mod recv_ops;
pub use self::recv_ops::*;

mod recvfrom_ops;
pub use self::recvfrom_ops::*;

mod resolve_ops;
pub use self::resolve_ops::*;

mod send_ops;
pub use self::send_ops::*;

mod sendto_ops;
pub use self::sendto_ops::*;

mod stream_ops;
pub use self::stream_ops::*;

mod wait_ops;
pub use self::wait_ops::*;

mod write_ops;
pub use self::write_ops::*;

mod signal_wait;
pub use self::signal_wait::*;

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
