use ffi::{Timeout};
use core::{AsIoContext, IoContext, ThreadIoContext, Cancel};
use handler::{Handler, Yield};
use strand::{Strand, StrandImpl, StrandImmutable, StrandHandler};
use SteadyTimer;

use std::raw;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use std::marker::PhantomData;
use context::{Context, Transfer};
use context::stack::{ProtectedFixedSizeStack, Stack, StackError};

trait CoroutineExec: Send + 'static {
    fn call_box(self: Box<Self>, coro: Coroutine);
}

impl<F> CoroutineExec for F
where
    F: FnOnce(Coroutine) + Send + 'static,
{
    fn call_box(self: Box<Self>, coro: Coroutine) {
        self(coro)
    }
}

pub struct CoroutineData {
    context: Option<Context>,
    timer: SteadyTimer
}

type Caller<R, E> = fn(Strand<CoroutineData>, Result<R, E>);

pub struct Callee<T> {
    data: Arc<StrandImpl<CoroutineData>>,
    _marker: PhantomData<T>,
}

impl<T> Yield<T> for Callee<T> {
    fn yield_wait(self, data: &Cancel) -> T {
        unsafe {
            let coro: &mut CoroutineData = &mut *self.data.cell.get();
            coro.timer.cancel();
            let cancel: raw::TraitObject = mem::transmute(data);
            let Transfer { context, data } = coro.context.take().unwrap().resume(&cancel as *const _ as usize);
            coro.context = Some(context);
            let res: &mut Option<T> = &mut *(data as *mut Option<T>);
            res.take().unwrap()
        }
    }

    fn yield_wait_for(self, data: &Cancel, timeout: &Timeout) -> T {
        unsafe {
            let coro: &mut CoroutineData = &mut *self.data.cell.get();
            coro.timer.expires_from_now(timeout.get());
            let cancel: raw::TraitObject = mem::transmute(data);
            let Transfer { context, data } = coro.context.take().unwrap().resume(&cancel as *const _ as usize);
            coro.context = Some(context);
            let res: &mut Option<T> = &mut *(data as *mut Option<T>);
            res.take().unwrap()
        }
    }
}

pub struct CoroutineHandler<R, E>(StrandHandler<CoroutineData, Caller<R, E>, R, E>);

impl<R, E> Handler<R, E> for CoroutineHandler<R, E>
where
    R: Send + 'static,
    E: Send + 'static,
{
    type Output = Result<R, E>;

    #[doc(hidden)]
    type Caller = StrandHandler<CoroutineData, fn(Strand<CoroutineData>, Result<R, E>), R, E>;

    #[doc(hidden)]
    type Callee = Callee<Self::Output>;

    #[doc(hidden)]
    fn channel(self) -> (Self::Caller, Self::Callee) {
        let data: Arc<StrandImpl<CoroutineData>> = self.0.data.clone();
        (
            self.0,
            Callee {
                data: data,
                _marker: PhantomData,
            },
        )
    }
}

struct InitData {
    stack: ProtectedFixedSizeStack,
    ctx: IoContext,
    exec: Box<CoroutineExec>,
}

/// Context object that represents the currently executing coroutine.
pub struct Coroutine<'a>(Strand<'a, CoroutineData>);

impl<'a> Coroutine<'a> {
    extern "C" fn entry(t: Transfer) -> ! {
        let InitData { stack, ctx, exec } = unsafe {
            &mut *(t.data as *mut Option<InitData>)
        }.take().unwrap();
        let mut coro: StrandImmutable<CoroutineData> = Strand::new(&ctx, CoroutineData {
            context: Some(t.context),
            timer: SteadyTimer::new(&ctx),
        });
        let this = {
            let data = &coro as *const _ as usize;
            let mut coro: &mut CoroutineData = unsafe { coro.get() };
            let Transfer { context, data } = unsafe { coro.context.take().unwrap().resume(data) };
            coro.context = Some(context);
            unsafe { &mut *(data as *mut ThreadIoContext) }
        };
        exec.call_box(Coroutine(coro.make_mut(this)));
        let context = (&mut unsafe { coro.get() }.context).take().unwrap();
        let mut stack = Some(stack);
        unsafe { context.resume_ontop(&mut stack as *mut _ as usize, Self::exit) };

        unreachable!();
    }

    extern "C" fn exit(mut t: Transfer) -> Transfer {
        {
            let stack = unsafe { &mut *(t.data as *mut Option<ProtectedFixedSizeStack>) };
            // Drop the stack
            let _ = stack.take().unwrap();
        }
        t.data = 0;
        t
    }

    /// Provides a `Coroutine` handler to asynchronous operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::{IoContext, AsIoContext, Stream, spawn};
    /// use asyncio::ip::{IpProtocol, Tcp, TcpSocket};
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// spawn(ctx, |coro| {
    ///   let ctx = coro.as_ctx();
    ///   let mut soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
    ///   let mut buf = [0; 256];
    ///   let size = soc.async_read_some(&mut buf, coro.wrap()).unwrap();
    /// });
    /// ```
    pub fn wrap<R, E>(&self) -> CoroutineHandler<R, E>
    where
        R: Send + 'static,
        E: Send + 'static,
    {
        let handler: StrandHandler<CoroutineData, Caller<R, E>, R, E> = self.0.wrap(caller::<R, E>);
        CoroutineHandler(handler)
    }
}

fn caller<R, E>(mut coro: Strand<CoroutineData>, res: Result<R, E>)
where
    R: Send + 'static,
    E: Send + 'static,
{
    let mut data = Some(res);
    let Transfer { context, data } =
        unsafe { coro.context.take().unwrap().resume(&mut data as *mut _ as usize) };
    if data != 0 {
        coro.context = Some(context);
        coro.timer.async_wait(coro.wrap(move |coro, res| {
            if let Ok(_) = res {
                let cancel: &Cancel = unsafe { mem::transmute(*(data as *const raw::TraitObject)) };
                cancel.cancel();
            }
        }));
    } else {
        coro.timer.cancel();
    }
}

unsafe impl<'a> AsIoContext for Coroutine<'a> {
    fn as_ctx(&self) -> &IoContext {
        self.0.as_ctx()
    }
}

pub fn spawn<F>(ctx: &IoContext, func: F) -> Result<(), StackError>
where
    F: FnOnce(Coroutine) + Send + 'static,
{
    let data = InitData {
        stack: ProtectedFixedSizeStack::new(Stack::default_size())?,
        ctx: ctx.clone(),
        exec: Box::new(func),
    };
    unsafe {
        let context = Context::new(&data.stack, Coroutine::entry);
        let data = Some(data);
        let Transfer { context, data } = context.resume(&data as *const _ as usize);
        let coro = &mut *(data as *mut StrandImmutable<CoroutineData>);
        coro.get().context = Some(context);
        coro.post(move |mut coro| {
            let data = coro.this as *mut _ as usize;
            let Transfer { context, data } = coro.context.take().unwrap().resume(data);
            if data != 0 {
                coro.context = Some(context);

                coro.timer.expires_from_now(Duration::new(10, 0));
                coro.timer.async_wait(coro.wrap(move |coro, res| {
                    if let Ok(_) = res {
                        let cancel: &Cancel = mem::transmute(*(data as *const raw::TraitObject));
                        cancel.cancel();
                    }
                }));
            }
        })
    }
    Ok(())
}

#[test]
fn test_spawn() {
    let ctx = &IoContext::new().unwrap();
    spawn(ctx, |coro| {});
    ctx.run();
}
