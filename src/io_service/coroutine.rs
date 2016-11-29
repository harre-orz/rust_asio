use std::io;
use std::boxed::FnBox;
use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;
use error::ErrCode;
use super::{IoObject, IoService, Strand, StrandImmutable, StrandHandler,
            Callback, Handler, AsyncResult, strand_clone};

struct InitData {
    stack: ProtectedFixedSizeStack,
    io: IoService,
    func: Box<FnBox(Coroutine)>,
}

extern "C" fn coro_entry(t: Transfer) -> ! {
    let InitData { stack, io, func } = unsafe {
        let data_opt_ref = &mut *(t.data as *mut Option<InitData>);
        data_opt_ref.take().unwrap()
    };

    let context = {
        let io = io;
        let coro = IoService::strand(&io, Some(t.context));
        let mut data = unsafe { coro.as_mut() };
        let Transfer { context, data:_ } = data.take().unwrap().resume(&coro as *const _ as usize);
        *data = Some(context);

        func.call_box((Coroutine(&data), ));
        data.take().unwrap()
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

fn coro_receiver<R: Send + 'static>(mut coro: Strand<Option<Context>>) -> R {
    let Transfer { context, data } = coro.take().unwrap().resume(0);
    *coro = Some(context);

    let data_opt = unsafe { &mut *(data as *mut Option<R>) };
    data_opt.take().unwrap()
}

fn coro_sender<R: Send + 'static>(mut coro: Strand<Option<Context>>, data: R) {
    let mut data_opt = Some(data);
    let Transfer { context, data } = coro.take().unwrap().resume(&mut data_opt as *mut _ as usize);
    if data == 0 {
        *coro = Some(context);
    }
}

/// Context object that represents the currently executing coroutine.
pub struct Coroutine<'a>(&'a Strand<'a, Option<Context>>);

impl<'a> Coroutine<'a> {
    /// Provides a `Coroutine` handler to asynchronous operation.
    ///
    /// The CoroutineHandler has trait the `Handler`, that type of `Handler::Output` is `io::Result<R>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::{IoObject, IoService, Stream};
    /// use asyncio::ip::{Tcp, TcpSocket};
    ///
    /// let io = &IoService::new();
    /// IoService::spawn(io, |coro| {
    ///   let io = coro.io_service();
    ///   let soc = TcpSocket::new(io, Tcp::v4()).unwrap();
    ///   let mut buf = [0; 256];
    ///   let size = soc.async_read_some(&mut buf, coro.wrap()).unwrap();
    /// });
    /// ```
    pub fn wrap<R: Send + 'static>(&self) -> CoroutineHandler<R> {
        CoroutineHandler(self.0.wrap(coro_sender))
    }
}

pub struct CoroutineAsyncResult<R>(Box<FnBox(*const IoService) -> R>);

impl<R> AsyncResult<R> for CoroutineAsyncResult<R> {
    fn get(self, io: &IoService) -> R {
        (self.0)(io)
    }
}

pub struct CoroutineHandler<R>(StrandHandler<Option<Context>, fn(Strand<Option<Context>>, io::Result<R>), R>);

impl<R: Send + 'static> Handler<R> for CoroutineHandler<R> {
    type Output = io::Result<R>;

    fn callback(self, io: &IoService, res: io::Result<R>) {
        debug_assert_eq!(self.0.data.is_ownered(), true);
        self.0.callback(io, res);
    }

    fn wrap<G>(self, callback: G) -> Callback
       where G: FnOnce(&IoService, ErrCode, Self) + Send + 'static
    {
        debug_assert_eq!(self.0.data.is_ownered(), true);
        Box::new(move |io: *const IoService, ec| {
            let io = unsafe { &*io };
            let data = self.0.data.clone();
            debug_assert_eq!(data.is_ownered(), false);
            data.dispatch(io, move|st| callback(st.io_service(), ec, self))
        })
    }

    type AsyncResult = CoroutineAsyncResult<Self::Output>;

    fn async_result(&self) -> Self::AsyncResult {
        let data = self.0.data.clone();
        debug_assert_eq!(data.is_ownered(), true);
        CoroutineAsyncResult(Box::new(move |io: *const IoService| -> Self::Output {
            debug_assert_eq!(data.is_ownered(), true);
            coro_receiver(strand_clone(unsafe { &*io }, &data))
        }))
    }
}

unsafe impl<'a> IoObject for Coroutine<'a> {
    fn io_service(&self) -> &IoService {
        self.0.io_service()
    }
}

pub fn spawn<F: FnOnce(Coroutine) + 'static>(io: &IoService, func: F) {
    let data = InitData {
        stack: ProtectedFixedSizeStack::default(),
        io: io.clone(),
        func: Box::new(func),
    };

    let context = Context::new(&data.stack, coro_entry);
    let mut data_opt = Some(data);
    let Transfer { context, data } = context.resume(&mut data_opt as *mut _ as usize);
    let coro = unsafe { &*(data as *const StrandImmutable<Option<Context>>) };
    *unsafe { coro.as_mut() } = Some(context);

    coro.post(move |mut coro| {
        let Transfer { context, data } = coro.take().unwrap().resume(0);
        if data == 0 {
            *coro = Some(context)
        }
    })
}

#[test]
fn test_spawn_0() {
    let io = &IoService::new();
    IoService::spawn(io, |_| {});
    io.run();
}

#[test]
fn test_spawn_1() {
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
