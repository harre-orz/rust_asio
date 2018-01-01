use unsafe_cell::UnsafeRefCell;
use error::ErrCode;
use core::{IoContext, AsIoContext, ThreadIoContext, FnOp, Upcast};
use async::{Handler, WrappedHandler, Receiver, Sender, Operation};
use async::strand::{StrandData, Strand, StrandImmutable, StrandHandler, strand_clone};

use std::marker::PhantomData;
use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;

trait FnBox {
    fn call_box(self: Box<Self>, Coroutine);
}

impl<F: FnOnce(Coroutine)> FnBox for F {
    fn call_box(self: Box<Self>, co: Coroutine) {
        (*self)(co)
    }
}

struct InitData {
    stack: ProtectedFixedSizeStack,
    ctx: IoContext,
    func: Box<FnBox>,
}

extern "C" fn coro_entry(t: Transfer) -> ! {
    let InitData { stack, ctx, func } = unsafe {
        let data_opt_ref = &mut *(t.data as *mut Option<InitData>);
        data_opt_ref.take().unwrap()
    };

    let context = {
        let ctx = ctx;
        let coro = IoContext::strand(&ctx, Some(t.context));
        let mut data = unsafe { coro.as_mut() };
        let Transfer { context, data:_ } = data.take().unwrap().resume(&coro as *const _ as usize);
        *data = Some(context);

        func.call_box(Coroutine(&data));
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
    /// use asyncio::{IoContext, AsIoContext, Stream};
    /// use asyncio::ip::{IpProtocol, Tcp, TcpSocket};
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// IoContext::spawn(ctx, |coro| {
    ///   let ctx = coro.as_ctx();
    ///   let mut soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
    ///   let mut buf = [0; 256];
    ///   let size = soc.async_read_some(&mut buf, coro.wrap()).unwrap();
    /// });
    /// ```
    pub fn wrap<R, E>(&self) -> CoroutineHandler<R, E>
        where R: Send + 'static,
              E: Send + 'static,
    {
        CoroutineHandler(self.0.wrap(coro_sender))
    }
}

unsafe impl<'a> AsIoContext for Coroutine<'a> {
    fn as_ctx(&self) -> &IoContext {
        self.0.as_ctx()
    }
}

pub struct CoroutineReceiver<R>(StrandData<Option<Context>>, PhantomData<R>);

impl<R: Send + 'static> Receiver<R> for CoroutineReceiver<R> {
    fn recv(self, ctx: &IoContext) -> R {
        coro_receiver(strand_clone(ctx, &self.0))
    }
}

pub struct CoroutineHandler<R, E>(
    StrandHandler<Option<Context>, fn(Strand<Option<Context>>, Result<R, E>), R, E>
);

impl<R, E> CoroutineHandler<R, E>
    where R: Send + 'static,
          E: Send + 'static,
{
    fn send(self, ctx: &IoContext, res: Result<R, E>) {
        self.0.send(ctx, res)
    }
}

impl<R, E> Handler<R, E> for CoroutineHandler<R, E>
    where R: Send + 'static,
          E: Send + 'static,
{
    type Output = Result<R, E>;

    type Receiver = CoroutineReceiver<Self::Output>;

    fn channel<G>(self, op: G) -> (Operation<R, E, G>, Self::Receiver)
        where G: WrappedHandler<R, E> + Send + 'static
    {
        let data = self.0.data.clone();
        (Box::new((self, op)), CoroutineReceiver(data, PhantomData))
    }

    fn result(self, _ctx: &IoContext, res: Result<R, E>) -> Self::Output {
        res
    }
}

impl<R, E, G> FnOp for (CoroutineHandler<R, E>, G)
    where R: Send + 'static,
          E: Send + 'static,
          G: WrappedHandler<R, E> + Send + 'static,
{
    fn call_op(self: Box<Self>, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode) {
        (self.0).0.data.clone().run(ctx, this, move|st: Strand<Option<Context>>, this: &mut ThreadIoContext| {
            let mut g = UnsafeRefCell::new(&self.1);
            unsafe { g.as_mut() }.perform(st.as_ctx(), this, ec, self)
        })
    }
}

impl<R, E, G> Upcast<FnOp + Send> for (CoroutineHandler<R, E>, G)
    where R: Send + 'static,
          E: Send + 'static,
          G: WrappedHandler<R, E> + Send + 'static,
{
    fn upcast(self: Box<Self>) -> Box<FnOp + Send> {
        self
    }
}

impl<R, E, G> Sender<R, E, G> for (CoroutineHandler<R, E>, G)
    where R: Send + 'static,
          E: Send + 'static,
          G: WrappedHandler<R, E> + Send + 'static,
{
    fn send(self: Box<Self>, ctx: &IoContext, res: Result<R, E>) {
        self.0.send(ctx, res)
    }

    fn as_self(&self) -> &G {
        &self.1
    }

    fn as_mut_self(&mut self) -> &mut G {
        &mut self.1
    }
}

impl IoContext {
    pub fn spawn<F>(ctx: &IoContext, func: F)
        where F: FnOnce(Coroutine) + 'static
    {
        let data = InitData {
            stack: ProtectedFixedSizeStack::default(),
            ctx: ctx.clone(),
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
}

#[test]
fn test_spawn_0() {
    let ctx = &IoContext::new().unwrap();
    IoContext::spawn(ctx, |_| {});
    ctx.run();
}

#[test]
fn test_spawn_1() {
    use ip::{IpProtocol, Udp, UdpSocket};

    let ctx = &IoContext::new().unwrap();
    IoContext::spawn(ctx, |coro| {
        let ctx = coro.as_ctx();
        let udp = UdpSocket::new(ctx, Udp::v4()).unwrap();
        let buf = [0; 256];
        assert!(udp.async_send(&buf, 0, coro.wrap()).is_err());
        assert!(udp.async_send(&buf, 0, coro.wrap()).is_err());
    });
    ctx.run();
}
