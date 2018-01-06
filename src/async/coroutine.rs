use core::{IoContext, AsIoContext, ThreadIoContext};
use async::{Yield, Complete, Handler, StrandHandler, Strand, StrandImpl, StrandImmutable};

use std::sync::Arc;
use std::marker::PhantomData;
use context::{Context, Transfer};
use context::stack::{Stack, ProtectedFixedSizeStack, StackError};


trait CoroutineTask : Send + 'static {
    fn call_box(self: Box<Self>, coro: Coroutine);
}

impl<F> CoroutineTask for F
    where F: FnOnce(Coroutine) + Send + 'static,
{
    fn call_box(self: Box<Self>, coro: Coroutine) {
        self(coro)
    }
}


pub struct CoroutineYield<T> {
    data: Arc<StrandImpl<Option<Context>>>,
    _marker: PhantomData<T>,
}

impl<T> Yield<T> for CoroutineYield<T> {
    fn yield_return(self, ctx: &IoContext) -> T {
        let Transfer { context, data } = unsafe { (&mut *self.data.cell.get()).take().unwrap().resume(0) };
        *unsafe { &mut *self.data.cell.get() } = Some(context);
        let data = unsafe { &mut *(data as *mut Option<T>) };
        data.take().unwrap()
    }
}


pub struct CoroutineHandler<R, E>(
    StrandHandler<Option<Context>, fn(Strand<Option<Context>>, Result<R, E>), R, E>
);

impl<R, E> Handler<R, E> for CoroutineHandler<R, E>
    where R: Send + 'static,
          E: Send + 'static,
{
    type Output = Result<R, E>;

    type Perform = StrandHandler<Option<Context>, fn(Strand<Option<Context>>, Result<R, E>), R, E>;

    type Yield = CoroutineYield<Self::Output>;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        let data = self.0.data.clone();
        (self.0, CoroutineYield { data: data, _marker: PhantomData })
    }
}


struct InitData {
    stack: ProtectedFixedSizeStack,
    ctx: IoContext,
    task: Box<CoroutineTask>,
}


/// Context object that represents the currently executing coroutine.
pub struct Coroutine<'a>(Strand<'a, Option<Context>>);

impl<'a> Coroutine<'a> {
    extern "C" fn entry(t: Transfer) -> ! {
        let InitData { stack, ctx, task } = unsafe {
            &mut *(t.data as *mut Option<InitData>)
        }.take().unwrap();

        let mut coro = Strand::new(&ctx, Some(t.context));
        let this = {
            let data = &coro as *const _ as usize;
            let mut coro = unsafe { coro.get() };
            let Transfer { context, data } = unsafe { coro.take().unwrap().resume(data) };
            *coro = Some(context);
                unsafe { &mut *(data as *mut ThreadIoContext) }
        };
        task.call_box(Coroutine(coro.make_mut(this)));
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

    fn send<R, E>(mut coro: Strand<Option<Context>>, res: Result<R, E>)
        where R: Send + 'static,
              E: Send + 'static,
    {
        let mut data = Some(res);
        let Transfer { context, data } = unsafe { coro.take().unwrap().resume(&mut data as *mut _ as usize) };
        if data == 0 {
            *coro = Some(context)
        }
    }

    pub fn wrap<R, E>(&self) -> CoroutineHandler<R, E>
        where R: Send + 'static,
              E: Send + 'static,
    {
        CoroutineHandler(self.0.wrap(Self::send::<R, E>))
    }

}

unsafe impl<'a> AsIoContext for Coroutine<'a> {
    fn as_ctx(&self) -> &IoContext {
        self.0.as_ctx()
    }
}


pub fn spawn<F>(ctx: &IoContext, func: F) -> Result<(), StackError>
    where F: FnOnce(Coroutine) + Send + 'static
{
    let data = InitData {
        stack: ProtectedFixedSizeStack::new(Stack::default_size())?,
        ctx: ctx.clone(),
        task: Box::new(func),
    };
    unsafe {
        let context = Context::new(&data.stack, Coroutine::entry);
        let mut data = Some(data);
        let Transfer { context, data } = context.resume(&mut data as *mut _ as usize);
        let data = &mut *(data as *mut StrandImmutable<Option<Context>>);
        *data.get() = Some(context);
        data.post(move |mut coro| {
            let data = coro.runnning_context() as *mut _ as usize;
            let Transfer { context, data } = coro.take().unwrap().resume(data);
            if data == 0 {
                *coro = Some(context);
            }
        });
    }
    Ok(())
}
