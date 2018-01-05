use super::{NoYield, Handler};
use core::{IoContext, AsIoContext, ThreadIoContext, Task};

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};


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
    type Perform = Self;

    #[doc(hidden)]
    type Yield = NoYield;

    #[doc(hidden)]
    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
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



trait StrandTask<T> : Send + 'static {
    fn call(self, this: &mut ThreadIoContext, data: &Arc<StrandImpl<T>>);

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext, data: &Arc<StrandImpl<T>>);
}

impl<T, F> StrandTask<T> for F
    where F: FnOnce(Strand<T>) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext, data: &Arc<StrandImpl<T>>) {
        self(Strand { ctx: this.as_ctx(), data: data })
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext, data: &Arc<StrandImpl<T>>) {
        self(Strand { ctx: this.as_ctx(), data: data })
    }
}


struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<Box<StrandTask<T>>>,
}


struct StrandImpl<T> {
    mutex: Mutex<StrandQueue<T>>,
    cell: UnsafeCell<T>,
}

impl<T> StrandImpl<T> {
    fn run<F>(this: &mut ThreadIoContext, data: Arc<StrandImpl<T>>, task: F)
        where T: 'static,
              F: StrandTask<T>,
    {
        {
            let mut owner = data.mutex.lock().unwrap();
            if owner.locked {
                owner.queue.push_back(Box::new(task));
                return;
            }
            owner.locked = true;
        }

        task.call(this, &data);

        while let Some(task) = {
            let mut owner = data.mutex.lock().unwrap();
            if let Some(task) = owner.queue.pop_front() {
                Some(task)
            } else {
                owner.locked = false;
                None
            }
        } {
            task.call_box(this, &data);
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
    where T: 'static,
          F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }

    fn complete(self, this: &mut ThreadIoContext, res: Result<R, E>) {
        let StrandHandler { data, handler, _marker } = self;
        StrandImpl::run(this, data, |strand: Strand<T>| handler(strand, res))
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
        owner.queue.push_back(Box::new(func))
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

unsafe impl<'a, T> AsIoContext for Strand<'a, T> {
    fn as_ctx(&self) -> &IoContext {
        self.ctx
    }

}


impl<T, F> Task for (Arc<StrandImpl<T>>, F)
    where T: 'static,
          F: FnOnce(Strand<T>) + Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        let (data, func) = self;
        StrandImpl::run(this, data, func)
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
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
        self.ctx.do_dispatch((data, func))
    }

    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static,
    {
        let data = self.data.clone();
        self.ctx.do_post((data, func))
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
