//

use super::{IoContext, TimerContext, ReactorContext};
use context_::{Context, Transfer};
use context_::stack::{ProtectedFixedSizeStack, Stack, StackError};
use std::ptr::NonNull;
use error::ErrorCode;

pub enum TransferData {
    Wait(NonNull<TimerContext>),
    Input(NonNull<ReactorContext>),
    Output(NonNull<ReactorContext>),
}

pub struct YieldContext {
    context: Option<Context>,
    io_ctx: IoContext,
}

impl YieldContext {
    // pub fn caller_wait(&mut self, timer_ctx: &mut TimerContext) {
    //     let context = self.context.take().unwrap();
    //     let data = TransferData::Wait(unsafe { NonNull::new_unchecked(timer_ctx) });
    //     let Transfer { context, data } = unsafe { context.resume(&data as *const _ as usize) };
    //     self.context = Some(context);
    // }
    //
    // pub fn caller_input(&mut self, socket_ctx: &mut ReactorContext) {
    //     let context = self.context.take().unwrap();
    //     let data = TransferData::Input(unsafe { NonNull::new_unchecked(socket_ctx) });
    //     let Transfer { context, data } = unsafe { context.resume(&data as *const _ as usize) };
    //     self.context = Some(context);
    // }
}


trait Exec {
    fn call_box(self: Box<Self>, yield_ctx: &mut YieldContext);
}

impl<F> Exec for F
    where F:
FnOnce(&mut YieldContext)
{
    fn call_box(self: Box<Self>, yield_ctx: &mut YieldContext) {
        self(yield_ctx)
    }
}

struct InitData {
    io_ctx: IoContext,
    stack: ProtectedFixedSizeStack,
    start: Box<dyn Exec>
}

extern "C" fn entry(t: Transfer) -> ! {
    let Transfer { context, data } = t;
    let data = unsafe { &mut *(data as *mut Option<InitData>) };
    let InitData { stack, io_ctx, start } = data.take().unwrap();
    let mut yield_ctx = YieldContext {
        context: Some(context),
        io_ctx: io_ctx,
    };
    start.call_box(&mut yield_ctx);
    let context = yield_ctx.context.take().unwrap();
    let mut stack = Some(stack);
    unsafe { context.resume_ontop(&mut stack as *mut _ as usize, exit) };
    unreachable!()
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

pub fn spawn<F>(io_ctx: &IoContext, start: F) -> Result<(), StackError>
    where F: FnOnce(&mut YieldContext) + 'static
{
    let data = InitData {
        io_ctx: io_ctx.clone(),
        stack: ProtectedFixedSizeStack::new(Stack::default_size())?,
        start: Box::new(start),
    };
    let context = unsafe {
        Context::new(&data.stack, entry)
    };
    let mut data = Some(data);
    let Transfer { context, data } = unsafe {
        context.resume(&mut data as *mut _ as usize)
    };
    //io_ctx.post(context, data);
    Ok(())
}
