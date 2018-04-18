use ffi::Timeout;
use core::{AsIoContext, IoContext, ThreadIoContext, Cancel};
use handler::{Handler, wrap};
use strand::{Strand, StrandImpl, StrandImmutable, StrandHandler};
use SteadyTimer;

use std::sync::Arc;
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
    timer: SteadyTimer,
}

unsafe impl AsIoContext for CoroutineData {
    fn as_ctx(&self) -> &IoContext {
        self.timer.as_ctx()
    }
}

type Caller<R, E> = fn(Strand<CoroutineData>, Result<R, E>);

pub struct CoroutineHandler<R, E>(StrandHandler<CoroutineData, Caller<R, E>, R, E>);

impl<R, E> Handler<R, E> for CoroutineHandler<R, E>
where
    R: Send + 'static,
    E: Send + 'static,
{
    type Output = Result<R, E>;

    #[doc(hidden)]
    type Handler = StrandHandler<CoroutineData, fn(Strand<CoroutineData>, Result<R, E>), R, E>;

    #[doc(hidden)]
    fn wrap<C, W>(self, ctx: &C, wrapper: W) -> Self::Output
    where
        C: AsIoContext,
        W: FnOnce(&IoContext, Self::Handler),
    {
        let data: Arc<StrandImpl<CoroutineData>> = self.0.data.clone();
        let coro: &mut CoroutineData = unsafe { &mut *data.cell.get() };
        wrapper(ctx.as_ctx(), self.0);
        let Transfer { context, data } = unsafe { coro.context.take().unwrap().resume(1) };
        coro.context = Some(context);
        coro.timer.cancel();
        let res: &mut Option<Self::Output> = unsafe { &mut *(data as *mut Option<Self::Output>) };
        res.take().unwrap()
    }

    // #[doc(hidden)]
    // fn wrap_timeout<C, W>(self, ctx: &C, timeout: Timeout, wrapper: W) -> Self::Output
    //     where C: Cancel,
    //           W: FnOnce(&IoContext, Self::Handler)
    // {
    //     use std::mem;
    //     use std::raw::TraitObject;
    //
    //     let data: Arc<StrandImpl<CoroutineData>> = self.0.data.clone();
    //     let coro: &mut CoroutineData = unsafe { &mut *data.cell.get() };
    //     wrapper(ctx.as_ctx(), self.0);
    //
    //     let data: (_, TraitObject) = (timeout, unsafe { mem::transmute(ctx as &Cancel) });
    //     let Transfer { context, data } = unsafe { coro.context.take().unwrap().resume(&data as *const _ as usize) };
    //     coro.context = Some(context);
    //     coro.timer.cancel();
    //     let res: &mut Option<Self::Output> = unsafe { &mut *(data as *mut Option<Self::Output>) };
    //     res.take().unwrap()
    // }
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
        let InitData { stack, ctx, exec } = unsafe { &mut *(t.data as *mut Option<InitData>) }
            .take()
            .unwrap();
        let mut coro: StrandImmutable<CoroutineData> = Strand::new(
            &ctx,
            CoroutineData {
                context: Some(t.context),
                timer: SteadyTimer::new(&ctx),
            },
        );
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
    use std::mem;
    use std::raw::TraitObject;

    struct TraitObjectSend(TraitObject);

    unsafe impl Send for TraitObjectSend {}

    let mut data = Some(res);
    let Transfer { context, data } = unsafe {
        coro.context.take().unwrap().resume(
            &mut data as *mut _ as usize,
        )
    };
    if data != 0 {
        if data != 1 {
            let data = unsafe { &*(data as *const (Timeout, TraitObject)) };
            coro.timer.expires_from_now(data.0.get());
            let raw = TraitObjectSend(data.1);
            coro.timer.async_wait(
                coro.wrap(move |ctx, res| if let Ok(_) = res {
                    let ctx: &Cancel = unsafe { mem::transmute(raw.0) };
                    ctx.cancel();
                }),
            );
        }
        coro.context = Some(context);
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
    let context = unsafe { Context::new(&data.stack, Coroutine::entry) };
    let data = Some(data);
    let Transfer { context, data } = unsafe { context.resume(&data as *const _ as usize) };
    let coro = unsafe { &mut *(data as *mut StrandImmutable<CoroutineData>) };
    unsafe { coro.get() }.context = Some(context);
    coro.post(move |mut coro| {
        let data = coro.this as *mut _ as usize;
        let Transfer { context, data } = unsafe { coro.context.take().unwrap().resume(data) };
        if data != 0 {
            coro.context = Some(context);
        }
    });
    Ok(())
}

#[test]
fn test_spawn() {
    let ctx = &IoContext::new().unwrap();
    spawn(ctx, |coro| {});
    ctx.run();
}
