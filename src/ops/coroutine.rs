use super::*;
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

type Caller<R, E> = fn(Strand<Option<Context>>, Result<R, E>);

pub struct Callee<T> {
    data: Arc<StrandImpl<Option<Context>>>,
    _marker: PhantomData<T>,
}

impl<T> Yield<T> for Callee<T> {
    fn yield_return(self) -> T {
        unsafe {
            let Transfer { context, data } = (&mut *self.data.cell.get()).take().unwrap().resume(0);
            *(&mut *self.data.cell.get()) = Some(context);
            (&mut *(data as *mut Option<T>)).take().unwrap()
        }
    }
}

pub struct CoroutineHandler<R, E>(StrandHandler<Option<Context>, Caller<R, E>, R, E>);

impl<R, E> Handler<R, E> for CoroutineHandler<R, E>
where
    R: Send + 'static,
    E: Send + 'static,
{
    type Output = Result<R, E>;

    #[doc(hidden)]
    type Caller = StrandHandler<Option<Context>, fn(Strand<Option<Context>>, Result<R, E>), R, E>;

    #[doc(hidden)]
    type Callee = Callee<Self::Output>;

    #[doc(hidden)]
    fn channel(self) -> (Self::Caller, Self::Callee) {
        let data = self.0.data.clone();
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
pub struct Coroutine<'a>(Strand<'a, Option<Context>>);

impl<'a> Coroutine<'a> {
    extern "C" fn entry(t: Transfer) -> ! {
        let InitData { stack, ctx, exec } = unsafe { &mut *(t.data as *mut Option<InitData>) }
            .take()
            .unwrap();

        let mut coro = Strand::new(&ctx, Some(t.context));
        let this = {
            let data = &coro as *const _ as usize;
            let mut coro = unsafe { coro.get() };
            let Transfer { context, data } = unsafe { coro.take().unwrap().resume(data) };
            *coro = Some(context);
            unsafe { &mut *(data as *mut ThreadIoContext) }
        };
        exec.call_box(Coroutine(coro.make_mut(this)));
        let context = unsafe { coro.get() }.take().unwrap();
        let mut stack = Some(stack);
        unsafe { context.resume_ontop(&mut stack as *mut _ as usize, Self::exit) };

        unreachable!();
    }

    extern "C" fn exit(t: Transfer) -> Transfer {
        {
            let stack = unsafe { &mut *(t.data as *mut Option<ProtectedFixedSizeStack>) };
            // Drop the stack
            let _ = stack.take().unwrap();
        }
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
        CoroutineHandler(self.0.wrap(caller::<R, E>))
    }
}

fn caller<R, E>(mut coro: Strand<Option<Context>>, res: Result<R, E>)
where
    R: Send + 'static,
    E: Send + 'static,
{
    let mut data = Some(res);
    let Transfer { context, data } =
        unsafe { coro.take().unwrap().resume(&mut data as *mut _ as usize) };
    if data == 0 {
        *coro = Some(context)
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
        let mut data = Some(data);
        let Transfer { context, data } = context.resume(&mut data as *mut _ as usize);
        let data = &mut *(data as *mut StrandImmutable<Option<Context>>);
        *data.get() = Some(context);
        data.post(|mut coro| {
            let data = coro.this as *mut _ as usize;
            let ::context::Transfer { context, data } = coro.take().unwrap().resume(data);
            if data == 0 {
                *coro = Some(context);
            }
        })
    }
    Ok(())
}
