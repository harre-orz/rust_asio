use unsafe_cell::UnsafeRefCell;
use error::ErrCode;
use core::{IoContext, ThreadIoContext, FnOp, Upcast};
use async::{Sender, NullReceiver, Operation, WrappedHandler, Handler};

use std::sync::Arc;
use std::marker::PhantomData;

/// The binding Arc handler.
pub struct ArcHandler<T, F, R, E> {
    data: Arc<T>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}

impl<T, F, R, E> ArcHandler<T, F, R, E>
    where T: Send + Sync + 'static,
          F: FnOnce(Arc<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
{
    fn send(self, _: &IoContext, res: Result<R, E>) {
        let ArcHandler { data, handler, _marker } = self;
        handler(data, res)
    }
}

impl<T, F, R, E> Handler<R, E> for ArcHandler<T, F, R, E>
    where T: Send + Sync + 'static,
          F: FnOnce(Arc<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
{
    type Output = ();

    type Receiver = NullReceiver;

    fn channel<G>(self, op: G) -> (Operation<R, E, G>, Self::Receiver)
        where G: WrappedHandler<R, E> + Send + 'static,
    {
        (Box::new((self, op)), NullReceiver)
    }

    fn result(self, _: &IoContext, res: Result<R, E>) -> Self::Output {
        let ArcHandler { data, handler, _marker } = self;
        handler(data, res)
    }
}

impl<T, F, R, E, G> Upcast<FnOp + Send> for (ArcHandler<T, F, R, E>, G)
    where T: Send + Sync + 'static,
          F: FnOnce(Arc<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
          G: WrappedHandler<R, E> + Send + 'static,
{
    fn upcast(self: Box<Self>) -> Box<FnOp + Send> {
        self
    }
}

impl<T, F, R, E, G> Sender<R, E, G> for (ArcHandler<T, F, R, E>, G)
    where T: Send + Sync + 'static,
          F: FnOnce(Arc<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
          G: WrappedHandler<R, E> + Send + 'static,
{
    fn send(self: Box<Self>, ctx: &IoContext, res: Result<R, E>) {
        ctx.post(move|ctx| self.0.send(ctx, res))
    }

    fn as_self(&self) -> &G {
        &self.1
    }

    fn as_mut_self(&mut self) -> &mut G {
        &mut self.1
    }
}

impl<T, F, R, E, G> FnOp for (ArcHandler<T, F, R, E>, G)
    where T: Send + Sync + 'static,
          F: FnOnce(Arc<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
          G: WrappedHandler<R, E> + Send + 'static,
{
    fn call_op(self: Box<Self>, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode) {
        let mut g = UnsafeRefCell::new(&self.1);
        unsafe { g.as_mut() }.perform(ctx, this, ec, self)
    }
}

/// Provides a `Arc` handler to asynchronous operation.
///
/// The ArcHandler has trait the `Handler`, that type of `Handler::Output` is `()`.
///
/// # Examples
///
/// ```
/// use std::io;
/// use std::sync::{Arc, Mutex};
/// use asyncio::{IoContext, wrap};
/// use asyncio::ip::{IpProtocol, Tcp, TcpSocket, TcpEndpoint, TcpListener};
///
/// fn on_accept(soc: Arc<Mutex<TcpListener>>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
///   if let Ok((acc, ep)) = res {
///     println!("accepted {}", ep)
///   }
/// }
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = Arc::new(Mutex::new(TcpListener::new(ctx, Tcp::v4()).unwrap()));
/// soc.lock().unwrap().async_accept(wrap(on_accept, &soc));
/// ```
pub fn wrap<T, F, R, E>(handler: F, data: &Arc<T>) -> ArcHandler<T, F, R, E> {
    ArcHandler {
        data: data.clone(),
        handler: handler,
        _marker: PhantomData,
    }
}
