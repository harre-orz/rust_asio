use std::io;
use std::mem;
use std::boxed::FnBox;
use std::sync::{Arc, Mutex, Barrier};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use {IoObject, IoService};
use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;
use async_result::{Handler, NullAsyncResult, BoxedAsyncResult};

/// The binding Arc<T> handler.
pub struct ArcHandler<T, F, R> {
    owner: Arc<T>,
    handler: F,
    marker: PhantomData<R>,
}

impl<T, F, R> Handler<R> for ArcHandler<T, F, R>
    where T: IoObject + Send + Sync + 'static,
          F: FnOnce(Arc<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static,
{
    type Output = ();

    #[doc(hidden)]
    type AsyncResult = NullAsyncResult;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult {
        NullAsyncResult
    }

    fn callback(self, _: &IoService, res: io::Result<R>) {
        let ArcHandler { owner, handler, marker:_ } = self;
        handler(owner, res)
    }
}

/// Provides a Arc<T> handler to asynchronous operation.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use asyncio::{IoService, bind};
/// use asyncio::ip::{Tcp, TcpSocket, TcpListener};
///
/// let io = &IoService::new();
/// let soc = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
/// soc.async_accept(bind(|soc, res| {
///   let _: Arc<TcpListener> = soc;
///
///   if let Ok((acc, ep)) = res {
///     let _: TcpSocket = acc;
///     println!("accepted {}", ep)
///   }
/// }, &soc));
/// ```
pub fn bind<T, F, R>(handler: F, owner: &Arc<T>) -> ArcHandler<T, F, R> {
    ArcHandler {
        owner: owner.clone(),
        handler: handler,
        marker: PhantomData,
    }
}

type ArcV<T> = Arc<(UnsafeStrandCell<T>, Mutex<StrandQueue<T>>)>;

type TaskHandler<T> = Box<FnBox(*const IoService, *const ArcV<T>) + Send + 'static>;

struct UnsafeStrandCell<T> {
    data: T,
}

impl<T> UnsafeStrandCell<T> {
    unsafe fn get(&self) -> &mut T {
        &mut *(&self.data as *const _ as *mut _)
    }
}

unsafe impl<T> Send for UnsafeStrandCell<T> {}

unsafe impl<T> Sync for UnsafeStrandCell<T> {}

struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<TaskHandler<T>>,
}

/// The binding Strand<T> handler.
pub struct StrandHandler<T, F, R> {
    value: ArcV<T>,
    handler: F,
    marker: PhantomData<R>,
}

impl<T, F, R> Handler<R> for StrandHandler<T, F, R>
    where T: Send + 'static,
          F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static,
{
    type Output = ();

    #[doc(hidden)]
    type AsyncResult = NullAsyncResult;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult {
        NullAsyncResult
    }

    fn callback(self, io: &IoService, res: io::Result<R>) {
        let StrandHandler { value, handler, marker:_ } = self;
        let _ = {
            let mut owner = value.1.lock().unwrap();
            if owner.locked {
                owner.queue.push_back(Box::new(move |io: *const IoService, value: *const ArcV<T>| {
                    let strand = Strand {
                        io: unsafe { &*io },
                        value: unsafe { &*value }.clone(),
                        is_new_object: false,
                    };
                    handler(strand, res)
                }));
                return;
            }
            owner.locked = true;
        };

        handler(Strand {
            io: io,
            value: value.clone(),
            is_new_object: false,
        }, res);

        while let Some(handler) = {
            let mut owner = value.1.lock().unwrap();
            if let Some(handler) = owner.queue.pop_front() {
                Some(handler)
            } else {
                owner.locked = false;
                None
            }
        } {
            handler(io, &value);
        }
    }
}

pub struct Strand<'a, T> {
    io: &'a IoService,
    value: ArcV<T>,
    is_new_object: bool,
}

impl<'a, T> Strand<'a, T> {
    pub fn new<U: IoObject>(io: &'a U, data: T) -> Strand<'a, T> {
        Strand {
            io: io.io_service(),
            value: Arc::new((UnsafeStrandCell { data: data }, Mutex::new(
                StrandQueue {
                    locked: true,
                    queue: VecDeque::new(),
                }
            ))),
            is_new_object: true,
        }
    }

    pub fn as_mut(&self) -> &mut T {
        unsafe { &mut *self.value.0.get() }
    }

    pub fn wrap<R, F>(&self, handler: F) -> StrandHandler<T, F, R>
        where F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
              R: Send + 'static,
    {
        StrandHandler {
            value: self.value.clone(),
            handler: handler,
            marker: PhantomData,
        }
    }
}

impl<'a, T> Drop for Strand<'a, T> {
    fn drop(&mut self) {
        if self.is_new_object {
            let mut val = self.value.1.lock().unwrap();
            val.locked = false;
        }
    }
}

impl<'a, T> IoObject for Strand<'a, T> {
    fn io_service(&self) -> &IoService {
        self.io
    }
}

impl<'a, T> Deref for Strand<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.value.0.get() }
    }
}

impl<'a, T> DerefMut for Strand<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.value.0.get() }
    }
}

struct CoroData {
    context: Option<Context>,
}

impl CoroData {
    fn receive<R>(&mut self) -> R {
        let Transfer { context, data } = self.context.take().unwrap().resume(0);
        self.context = Some(context);
        let data_opt = unsafe { &mut *(data as *mut Option<R>) };
        data_opt.take().unwrap()
    }

    fn send<R>(&mut self, data: R) {
        let mut data_opt = Some(data);
        let Transfer { context, data:_ } = self.context.take().unwrap().resume(&mut data_opt as *mut _ as usize);
        self.context = Some(context);
    }
}

pub struct Coroutine<'a>(Strand<'a, CoroData>);

impl<'a> Coroutine<'a> {
    pub fn yield_with<R: Send + 'static>(&self) -> CoroutineHandler<R> {
        fn coro_sender<R: Send + 'static>(mut coro: Strand<CoroData>, res: io::Result<R>) {
            coro.send(res)
        }

        CoroutineHandler {
            handler: self.0.wrap(coro_sender),
            barrier: Arc::new(Barrier::new(1)),
        }
    }
}

impl<'a> IoObject for Coroutine<'a> {
    fn io_service(&self) -> &IoService {
        self.0.io_service()
    }
}

struct InitData {
    stack: ProtectedFixedSizeStack,
    io: IoService,
    callback: Box<FnBox(&Coroutine)>,
}

pub struct CoroutineHandler<R> {
    handler: StrandHandler<CoroData, fn(Strand<CoroData>, io::Result<R>), R>,
    barrier: Arc<Barrier>,
}

impl<R: Send + 'static> Handler<R> for CoroutineHandler<R> {
    type Output = io::Result<R>;

    #[doc(hidden)]
    type AsyncResult = BoxedAsyncResult<Self::Output>;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult {
        let value = self.handler.value.clone();
        let barrier = self.barrier.clone();
        BoxedAsyncResult::new(move |io| -> Self::Output {
            let mut coro = Strand {
                io: io,
                value: value,
                is_new_object: false,
            };
            barrier.wait();
            coro.receive()
        })
    }

    fn callback(self, io: &IoService, res: io::Result<R>) {
        self.handler.callback(io, res)
    }
}

extern "C" fn coro_entry(t: Transfer) -> ! {
    let InitData { stack, io, callback } = unsafe {
        let data_opt_ref = &mut *(t.data as *mut Option<InitData>);
        data_opt_ref.take().unwrap()
    };

    // TODO: io と callback は使わないで StrandHandler を使うようにする
    let mut coro = Coroutine({
        let st = Strand::new(&io, CoroData { context: None });
        Strand {
            io: &io,
            value: st.value.clone(),
            is_new_object: false,
        }
    });

    let context = {
        let coro_ref = &mut coro as *mut _ as usize;
        let Transfer { context, data:_ } = t.context.resume(coro_ref);

        let coro_ref = unsafe { &mut *(coro_ref as *mut Coroutine) };
        coro_ref.0.context = Some(context);

        callback.call_box((coro_ref, ));
        coro_ref.0.context.take().unwrap()
    };

    let mut stack_opt = Some(stack);
    context.resume_ontop(&mut stack_opt as *mut _ as usize, coro_exit);

    unreachable!();
}

extern "C" fn coro_exit(mut t: Transfer) -> Transfer {
    unsafe {
        // Drop the stack
        let stack_ref = &mut *(t.data as *mut Option<ProtectedFixedSizeStack>);
        let _ = stack_ref.take().unwrap();
        t.context = { mem::transmute(0usize) };
    }
    t
}

pub fn spawn<T: IoObject, F: FnOnce(&Coroutine) + 'static>(io: &T, callback: F) {
    let io = io.io_service();
    let data = InitData {
        stack: Default::default(),
        io: io.clone(),
        callback: Box::new(callback),
    };

    let context = Context::new(&data.stack, coro_entry);
    let mut data_opt = Some(data);
    let data_opt_ref = &mut data_opt as *mut _ as usize;
    let t = context.resume(data_opt_ref);

    let coro_ref = unsafe { &mut *(t.data as *mut Coroutine) };
    coro_ref.0.context = Some(t.context);

    fn coro_handler(mut coro: Strand<CoroData>, _: io::Result<()>) {
        let Transfer { context, data:_ } = coro.context.take().unwrap().resume(0);
        coro.context = Some(context);
    }
    let handler = coro_ref.0.wrap(coro_handler);
    io.post(move |io| handler.callback(io, Ok(())));
}


// #[test]
// fn test_strand_race_condition() {
//     use std::time::Duration;
//     use std::thread;

//     let io = &IoService::new();
//     io.work(|io| {
//         io.post(|io| {
//             let st = Strand::new(io, 0);
//             assert_eq!(*st, 0);

//             let wrap = st.wrap(|mut st, _| {
//                 *st = 1;
//                 st.io_service().stop();
//             });
//             io.post(move |io| wrap.callback(io, Ok(())));
//             thread::sleep(Duration::from_secs(1));
//             assert_eq!(*st, 0);
//         });

//         let io = io.clone();
//         thread::spawn(move || io.run());
//     });
// }
