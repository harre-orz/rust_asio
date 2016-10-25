use std::io;
use std::boxed::FnBox;
use std::sync::{Arc, Barrier};
use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;
use super::{IoObject, IoService, Strand, StrandImpl, StrandHandler, Handler, BoxedAsyncResult, strand};

const DEFAULT_STACK_SIZE: usize = 2 * 1024 * 1024; // 2M

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

    let imp = StrandImpl::new(None, false);
    let mut coro = Coroutine(strand(&io, &imp));
    let context = {
        let coro_ref = &mut coro as *mut _ as usize;
        let Transfer { context, data:_ } = t.context.resume(coro_ref);

        let coro_ref = unsafe { &mut *(coro_ref as *mut Coroutine) };
        *coro_ref.0 = Some(context);
        func.call_box((coro_ref, ));
        coro_ref.0.take().unwrap()
    };

    let mut stack_opt = Some(stack);
    context.resume_ontop(&mut stack_opt as *mut _ as usize, coro_exit);

    unreachable!();
}

extern "C" fn coro_exit(t: Transfer) -> Transfer {
    unsafe {
        // Drop the stack
        let stack_opt_ref = &mut *(t.data as *mut Option<ProtectedFixedSizeStack>);
        let _ = stack_opt_ref.take().unwrap();
    }
    t
}

pub struct Coroutine<'a>(Strand<'a, Option<Context>>);


impl<'a> Coroutine<'a> {
    /// Returns a `Coroutine` handler to asynchronous operation.
    pub fn wrap<R: Send + 'static>(&self) -> CoroutineHandler<R> {
        CoroutineHandler {
            handler: self.0.wrap(coro_sender),
            barrier: Arc::new(Barrier::new(1)),
        }
    }
}

pub struct CoroutineHandler<R> {
    handler: StrandHandler<Option<Context>, fn(Strand<Option<Context>>, io::Result<R>), R>,
    barrier: Arc<Barrier>,
}

fn coro_receiver<R: Send + 'static>(mut coro: Strand<Option<Context>>) -> R {
    println!("receiver beg");
    let Transfer { context, data } = coro.take().unwrap().resume(0);
    println!("receiver end {}", data);

    *coro = Some(context);
    let data_opt = unsafe { &mut *(data as *mut Option<R>) };
    data_opt.take().unwrap()
}

fn coro_sender<R: Send + 'static>(mut coro: Strand<Option<Context>>, data: R) {
    let mut data_opt = Some(data);
    println!("sender beg");
    let Transfer { context, data } = coro.take().unwrap().resume(&mut data_opt as *mut _ as usize);
    println!("sender end {}", data);
    if data == 0 {
        *coro = Some(context);
    }
}

impl<R: Send + 'static> Handler<R> for CoroutineHandler<R> {
    type Output = io::Result<R>;

    type AsyncResult = BoxedAsyncResult<Self::Output>;

    fn async_result(&self) -> Self::AsyncResult {
        let barrier = self.barrier.clone();
        let imp = self.handler.imp.clone();
        BoxedAsyncResult::new(move |io| -> Self::Output {
            barrier.wait();
            coro_receiver(strand(io, &imp))
        })
    }

    fn callback(self, io: &IoService, res: io::Result<R>) {
        self.handler.callback(io, res);
    }
}

unsafe impl<'a> IoObject for Coroutine<'a> {
    fn io_service(&self) -> &IoService {
        self.0.io_service()
    }
}

pub fn spawn<F: FnOnce(&Coroutine) + 'static>(io: &IoService, func: F) {
    let data = InitData {
        io: io.clone(),
        stack: ProtectedFixedSizeStack::new(DEFAULT_STACK_SIZE).unwrap(),
        func: Box::new(func),
    };

    let context = Context::new(&data.stack, coro_entry);
    let mut data_opt = Some(data);
    let data_opt_ref = &mut data_opt as *mut _ as usize;
    let t = context.resume(data_opt_ref);

    let coro_ref = unsafe { &mut *(t.data as *mut Coroutine) };
    *coro_ref.0 = Some(t.context);

    fn coro_handler(mut coro: Strand<Option<Context>>, _: io::Result<()>) {
        let Transfer { context, data } = coro.take().unwrap().resume(0);
        if data == 0 {
            *coro = Some(context);
        }
    }
    let handler = coro_ref.0.wrap(coro_handler);
    io.post(move |io| handler.callback(io, Ok(())));
}

#[test]
fn test_spawn() {
    use ip::{Udp, UdpSocket};
    let io = &IoService::new();
    IoService::spawn(io, |coro| {
        let io = coro.io_service();
        let udp = UdpSocket::new(io, Udp::v4()).unwrap();
        let buf = [0; 256];
        assert!(udp.async_send(&buf, 0, coro.wrap()).is_err());
        assert!(udp.async_send(&buf, 0, coro.wrap()).is_err());
    });
    io.run();
}
