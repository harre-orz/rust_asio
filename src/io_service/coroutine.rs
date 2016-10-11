use std::io;
use std::mem;
use std::boxed::FnBox;
use std::sync::{Arc, Barrier};
use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;
use io_service::{IoObject, IoService, Strand, StrandHandler};
use async_result::{Handler, BoxedAsyncResult};

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
            coro_receiver(Strand { io: io, owner: owner })
        })
    }

    fn callback(self, io: &IoService, res: io::Result<R>) {
        self.handler.callback(io, res)
    }
}

pub struct Coroutine<'a>(Strand<'a, Option<Context>>);

impl<'a> Coroutine<'a> {
    pub fn yield_with<R: Send + 'static>(&self) -> CoroutineHandler<R> {
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
    unsafe {
        // Drop the stack
        let stack_ref = &mut *(t.data as *mut Option<ProtectedFixedSizeStack>);
        let _ = stack_ref.take().unwrap();
        t.context = { mem::transmute(0usize) };
    }
    t
}

pub fn spawn<T: IoObject, F: FnOnce(&Coroutine) + 'static>(io: &T, func: F) {
    let io = io.io_service();
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
