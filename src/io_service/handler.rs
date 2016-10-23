use std::io;
use std::boxed::FnBox;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::marker::PhantomData;
use std::collections::VecDeque;
use unsafe_cell::{UnsafeStrandCell};
use super::{IoObject, IoService};

pub trait AsyncResult<R> {
    fn get(self, io: &IoService) -> R;
}

pub struct NoAsyncResult;

impl AsyncResult<()> for NoAsyncResult {
    fn get(self, _io: &IoService) {
    }
}

pub trait Handler<R> : Send + 'static {
    type Output;

    fn callback(self, io: &IoService, res: io::Result<R>);

    #[doc(hidden)]
    type AsyncResult : AsyncResult<Self::Output>;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult;
}


/// The binding Arc handler.
pub struct ArcHandler<T, F, R> {
    owner: Arc<T>,
    handler: F,
    _marker: PhantomData<R>,
}

impl<T, F, R> Handler<R> for ArcHandler<T, F, R>
    where T: IoObject + Send + Sync + 'static,
          F: FnOnce(Arc<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static,
{
    type Output = ();

    fn callback(self, _: &IoService, res: io::Result<R>) {
        let ArcHandler { owner, handler, _marker } = self;
        handler(owner, res)
    }

    #[doc(hidden)]
    type AsyncResult = NoAsyncResult;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult {
        NoAsyncResult
    }
}

/// Provides a Arc<T> handler to asynchronous operation.
///
/// # Examples
///
/// ```
/// use std::io;
/// use std::sync::Arc;
/// use asyncio::{IoService, wrap};
/// use asyncio::ip::{Tcp, TcpSocket, TcpEndpoint, TcpListener};
///
/// fn on_accept(soc: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
///   if let Ok((acc, ep)) = res {
///     println!("accepted {}", ep)
///   }
/// }
///
/// let io = &IoService::new();
/// let soc = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
/// soc.async_accept(wrap(on_accept, &soc));
/// ```
pub fn wrap<T, F, R>(handler: F, owner: &Arc<T>) -> ArcHandler<T, F, R>
    where T: IoObject,
{
    ArcHandler {
        owner: owner.clone(),
        handler: handler,
        _marker: PhantomData,
    }
}


type Function<T> = Box<FnBox(*const IoService, StrandData<T>) + Send + 'static>;

struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<Function<T>>,
}

struct StrandData<T>(Arc<(UnsafeStrandCell<T>, Mutex<StrandQueue<T>>)>);

/// The binding Strand handler.
pub struct StrandHandler<T, F, R> {
    owner: StrandData<T>,
    handler: F,
    _marker: PhantomData<R>,
}

pub struct Strand<'a, T> {
    io: &'a IoService,
    owner: StrandData<T>,
}

impl<T, F, R> Handler<R> for StrandHandler<T, F, R>
    where T: Send + 'static,
          F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static
{
    type Output = ();

    fn callback(self, io: &IoService, res: io::Result<R>) {
        let StrandHandler { owner, handler, _marker } = self;
        Strand { io: io, owner: owner }.dispatch(move |io| handler(io, res));
    }

    #[doc(hidden)]
    type AsyncResult = NoAsyncResult;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult {
        NoAsyncResult
    }
}

impl<T> Clone for StrandData<T> {
    fn clone(&self) -> StrandData<T> {
        StrandData(self.0.clone())
    }
}

impl<'a, T: 'static> Strand<'a, T> {
    pub fn new(io: &'a IoService, data: T) -> Strand<'a, T> {
        Strand {
            io: io,
            owner: StrandData(Arc::new((UnsafeStrandCell::new(data), Mutex::new(
                StrandQueue {
                    locked: false,
                    queue: VecDeque::new(),
                }
            ))))
        }
    }

    pub unsafe fn get(&self) -> &mut T {
        &mut *(self.owner.0).0.get()
    }

    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let _ = {
            let mut owner = (self.owner.0).1.lock().unwrap();
            if owner.locked {
                owner.queue.push_back(Box::new(move |io: *const IoService, owner: StrandData<T>| {
                    func(Strand { io: unsafe { &*io }, owner: owner })
                }));
                return;
            }
            owner.locked = true;
        };

        func(Strand { io: self.io, owner: self.owner.clone() });

        while let Some(func) = {
            let mut owner = (self.owner.0).1.lock().unwrap();
            if let Some(func) = owner.queue.pop_front() {
                Some(func)
            } else {
                owner.locked = false;
                None
            }
        } {
            func(self.io, self.owner.clone());
        }
    }

    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let owner = self.owner.clone();
        self.io.post(move |io| Strand { io: io, owner: owner }.dispatch(func));
    }

    pub fn wrap<F, R>(&self, handler: F) -> StrandHandler<T, F, R>
        where F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
              R: Send + 'static,
    {
        StrandHandler {
            owner: self.owner.clone(),
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<'a, T> IoObject for Strand<'a, T> {
    fn io_service(&self) -> &IoService {
        self.io
    }
}

impl<'a, T> Deref for Strand<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.owner.0).0.get() }
    }
}

impl<'a, T> DerefMut for Strand<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.owner.0).0.get() }
    }
}

#[cfg(feature = "context")]
mod coroutine {
    use std::io;
    use std::mem;
    use std::boxed::FnBox;
    use std::sync::{Arc, Barrier};
    use context::{Context, Transfer};
    use context::stack::ProtectedFixedSizeStack;
    use super::super::{IoObject, IoService};
    use super::*;

    pub struct BoxedAsyncResult<R>(Box<FnBox(*const IoService) -> R>);

    impl<R> BoxedAsyncResult<R> {
        pub fn new<F>(func: F) -> BoxedAsyncResult<R>
            where F: FnOnce(&IoService) -> R + 'static
        {
            BoxedAsyncResult(Box::new(|io: *const IoService| func(unsafe { &*io })))
        }
    }

    impl<R> AsyncResult<R> for BoxedAsyncResult<R> {
        fn get(self, io: &IoService) -> R {
            (self.0)(io)
        }
    }

    fn coro_receiver<R: Send + 'static>(mut coro: Strand<Option<Context>>) -> R {
        let Transfer { context, data } = coro.take().unwrap().resume(0);
        *coro = Some(context);
        let data_opt = unsafe { &mut *(data as *mut Option<R>) };
        data_opt.take().unwrap()
    }

    fn coro_sender<R: Send + 'static>(mut coro: Strand<Option<Context>>, data: R) {
        let mut data_opt = Some(data);
        let Transfer { context, data:_ } = coro.take().unwrap().resume(&mut data_opt as *mut _ as usize);
        *coro = Some(context)
    }

    pub struct CoroutineHandler<R> {
        handler: StrandHandler<Option<Context>, fn(Strand<Option<Context>>, io::Result<R>), R>,
        barrier: Arc<Barrier>,
    }

    impl<R: Send + 'static> Handler<R> for CoroutineHandler<R> {
        type Output = io::Result<R>;

        type AsyncResult = BoxedAsyncResult<Self::Output>;

        fn async_result(&self) -> Self::AsyncResult {
            let owner = self.handler.owner.clone();
            let barrier = self.barrier.clone();
            BoxedAsyncResult::new(move |io| -> Self::Output {
                barrier.wait();
                println!("coroutine result");
                coro_receiver(Strand { io: io, owner: owner })
            })
        }

        fn callback(self, io: &IoService, res: io::Result<R>) {
            println!("coroutine callback");
            self.handler.callback(io, res)
        }
    }

    pub struct Coroutine<'a>(Strand<'a, Option<Context>>);

    impl<'a> Coroutine<'a> {
        pub fn wrap<R: Send + 'static>(&self) -> CoroutineHandler<R> {
            CoroutineHandler {
                handler: self.0.wrap(coro_sender),
                barrier: Arc::new(Barrier::new(1)),
            }
        }
    }

    unsafe impl<'a> IoObject for Coroutine<'a> {
        fn io_service(&self) -> &IoService {
            self.0.io_service()
        }
    }

    struct InitData {
        io: IoService,
        stack: ProtectedFixedSizeStack,
        func: Box<FnBox(&Coroutine)>,
    }

    extern "C" fn coro_entry(t: Transfer) -> ! {
        let InitData { io, stack, func } = unsafe {
            let data_opt_ref = &mut *(t.data as *mut Option<InitData>);
            data_opt_ref.take().unwrap()
        };

        let mut coro = Coroutine(Strand::new(&io, None));
        let context = {
            let coro_ref = &mut coro as *mut _ as usize;
            let Transfer { context, data:_ } = t.context.resume(coro_ref);

            let coro_ref = unsafe { &mut *(coro_ref as *mut Coroutine) };
            *coro_ref.0 =  Some(context);
            func.call_box((coro_ref, ));
            coro_ref.0.take().unwrap()
        };

        let mut stack_opt = Some(stack);
        context.resume_ontop(&mut stack_opt as *mut _ as usize, coro_exit);

        unreachable!();
    }

    extern "C" fn coro_exit(mut t: Transfer) -> Transfer {
        println!("coro exit");
        unsafe {
            // Drop the stack
            let stack_ref = &mut *(t.data as *mut Option<ProtectedFixedSizeStack>);
            let _ = stack_ref.take().unwrap();
            t.context = { mem::transmute(0usize) };
        }
        t
    }

    pub fn spawn<F: FnOnce(&Coroutine) + 'static>(io: &IoService, func: F) {
        let data = InitData {
            stack: Default::default(),
            io: io.clone(),
            func: Box::new(func),
        };

        let context = Context::new(&data.stack, coro_entry);
        let mut data_opt = Some(data);
        let data_opt_ref = &mut data_opt as *mut _ as usize;
        let t = context.resume(data_opt_ref);

        let coro_ref = unsafe { &mut *(t.data as *mut Coroutine) };
        *coro_ref.0 = Some(t.context);

        fn coro_handler(mut coro: Strand<Option<Context>>, _: io::Result<()>) {
            let Transfer { context, data:_ } = coro.take().unwrap().resume(0);
            *coro = Some(context);
        }
        let handler = coro_ref.0.wrap(coro_handler);
        io.post(move |io| handler.callback(io, Ok(())));
    }
}
#[cfg(feature = "context")] pub use self::coroutine::*;
