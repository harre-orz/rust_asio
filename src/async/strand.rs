use super::{NoYield, Handler};
use core::{IoContext, AsIoContext, ThreadIoContext, Task};

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};


pub struct WrappedHandler<F, R, E>(F, PhantomData<(R, E)>);

impl<F, R, E> Task for WrappedHandler<F, R, E>
    where F: Handler<R, E>,
          R: Send + 'static,
          E: Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {}

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {}
}

impl<F, R, E> Handler<R, E> for WrappedHandler<F, R, E>
    where F: Handler<R, E>,
          R: Send + 'static,
          E: Send + 'static,
{
    type Output = ();

    #[doc(hidden)]
    type Perform = Self;

    #[doc(hidden)]
    type Yield = NoYield;

    #[doc(hidden)]
    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }

    fn complete(self, this: &mut ThreadIoContext, res: Result<R, E>) {
        self.0.complete(this, res)
    }

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: R) {
        self.complete(this, Ok(res))
    }

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: E) {
        self.complete(this, Err(err))
    }
}


pub struct ArcHandler<T, F, R, E> {
    data: Arc<T>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}


impl<T, F, R, E> Handler<R, E> for ArcHandler<T, F, R, E>
    where T: Send + Sync + 'static,
          F: FnOnce(Arc<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
{
    type Output = ();

    #[doc(hidden)]
    type Perform = WrappedHandler<Self, R, E>;

    #[doc(hidden)]
    type Yield = NoYield;

    #[doc(hidden)]
    fn channel(self) -> (Self::Perform, Self::Yield) {
        (WrappedHandler(self, PhantomData), NoYield)
    }

    fn complete(self, this: &mut ThreadIoContext, res: Result<R, E>) {
        let ArcHandler { data, handler, _marker } = self;
        handler(data, res)
    }

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: R) {
        self.complete(this, Ok(res))
    }

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: E) {
        self.complete(this, Err(err))
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
/// fn on_accept(soc: Arc<Mutex<TcpListener>>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
///   if let Ok((acc, ep)) = res {
///     println!("accepted {}", ep)
///   }
/// }
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = Arc::new(Mutex::new(TcpListener::new(ctx, Tcp::v4()).unwrap()));
/// soc.lock().unwrap().async_accept(wrap(on_accept, &soc));
/// ```
pub fn wrap<T, F, R, E>(handler: F, data: &Arc<T>) -> ArcHandler<T, F, R, E> {
    ArcHandler {
        data: data.clone(),
        handler: handler,
        _marker: PhantomData,
    }
}


trait StrandFunc<T> : Send + 'static {
    fn call_box(self: Box<Self>, strand: Strand<T>);
}

struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<Box<StrandFunc<T>>>,
}


struct StrandImpl<T> {
    mutex: Mutex<StrandQueue<T>>,
    cell: UnsafeCell<T>,
}

impl<T> StrandImpl<T> {
    fn run<F>(data: Arc<StrandImpl<T>>, this: &mut ThreadIoContext, func: F)
        where T: 'static,
              F: FnOnce(Strand<T>) + Send + 'static,
    {
        // lock-guard
        {
            let mut owner = data.mutex.lock().unwrap();
            if owner.locked {
                //owner.queue.push_back(box func);
                return;
            }
            owner.locked = true;
        }

        func(Strand { ctx: this.as_ctx(), data: &data });

        while let Some(func) = {
            let mut owner = data.mutex.lock().unwrap();
            if let Some(func) = owner.queue.pop_front() {
                Some(func)
            } else {
                owner.locked = false;
                None
            }
        } {
            func.call_box(Strand { ctx: this.as_ctx(), data: &data });
        }
    }
}

unsafe impl<T> Send for StrandImpl<T> {}

unsafe impl<T> Sync for StrandImpl<T> {}


struct TaskOnce<T, F>(Arc<StrandImpl<T>>, F);

impl<T, F> Task for TaskOnce<T, F>
    where T: Send + 'static,
          F: FnOnce(Strand<T>) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        let TaskOnce(data, handler) = self;
        StrandImpl::run(data, this, handler);
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

pub struct StrandHandler<T, F, R, E> {
    data: Arc<StrandImpl<T>>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}

impl<T, F, R, E> Handler<R, E> for StrandHandler<T, F, R, E>
    where T: 'static,
          F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
{
    type Output = ();

    type Perform = WrappedHandler<Self, R, E>;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (WrappedHandler(self, PhantomData), NoYield)
    }

    fn complete(self, this: &mut ThreadIoContext, res: Result<R, E>) {
        let StrandHandler { data, handler, _marker } = self;
        //StrandImpl::run(data, this, handler)
    }

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: R) {
        self.complete(this, Ok(res))
    }

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: E) {
        self.complete(this, Err(err))
    }
}


pub struct Strand<'a, T: 'a> {
    ctx: &'a IoContext,
    data: &'a Arc<StrandImpl<T>>,
}

impl<'a, T> Strand<'a, T> {
    pub fn get(&self) -> &mut T {
        unsafe { &mut *self.data.cell.get() }
    }

    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        func(Strand { ctx: self.ctx, data: self.data })
    }

    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let mut owner = self.data.mutex.lock().unwrap();
        //owner.queue.push_back(box func)
    }

    pub fn wrap<F, R, E>(&self, handler: F) -> StrandHandler<T, F, R, E>
        where F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
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

impl<'a, T> Strand<'a, T> {
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
}


pub struct StrandImmutable<'a, T> {
    ctx: &'a IoContext,
    data: Arc<StrandImpl<T>>,
}

impl<'a, T> StrandImmutable<'a, T>
    where T: 'static,
{
    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static,
    {
        let data = self.data.clone();
        //self.ctx.do_dispatch(TaskOnce(data, func))
    }

    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static,
    {
        let data = self.data.clone();
        //self.ctx.do_post(TaskOnce(data, func))
    }

    pub unsafe fn as_mut(&'a self) -> Strand<'a, T> {
        Strand {
            ctx: self.ctx,
            data: &self.data,
        }
    }
}
