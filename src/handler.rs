use ffi::{SystemError};
use core::{IoContext, AsIoContext, Exec, Perform, ThreadIoContext};

use std::sync::Arc;
use std::marker::PhantomData;

pub trait Complete<R, E>: Send + 'static {
    fn success(self, this: &mut ThreadIoContext, res: R);

    fn failure(self, this: &mut ThreadIoContext, err: E);
}

pub trait Handler<R, E>: Send + 'static {
    type Output;

    #[doc(hidden)]
    type Handler: Complete<R, E>;

    #[doc(hidden)]
    fn wrap<W>(self, ctx: &IoContext, wrapper: W) -> Self::Output
        where W: FnOnce(&IoContext, Self::Handler);
}

pub trait AsyncReadOp: AsIoContext + Send + 'static {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn next_read_op(&self, this: &mut ThreadIoContext);
}

pub trait AsyncWriteOp: AsIoContext + Send + 'static {
    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn next_write_op(&self, this: &mut ThreadIoContext);
}

pub struct Failure<T, F, R, E>(T, F, PhantomData<(R, E)>);

impl<T, F, R, E> Failure<T, F, R, E> {
    pub fn new(err: T, handler: F) -> Self {
        Failure(err, handler, PhantomData)
    }
}

impl<T, F, R, E> Exec for Failure<T, F, R, E>
where
    T: Into<E> + Send + 'static,
    F: Complete<R, E>,
    R: Send + 'static,
    E: Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        let Failure(err, handler, _marker) = self;
        handler.failure(this, err.into())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

pub struct ArcHandler<T, F, R, E> {
    data: Arc<T>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}

impl<T, F, R, E> Handler<R, E> for ArcHandler<T, F, R, E>
where
    T: AsIoContext + Send + Sync + 'static,
    F: FnOnce(Arc<T>, Result<R, E>)
        + Send
        + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    type Output = ();

    #[doc(hidden)]
    type Handler = Self;

    #[doc(hidden)]
    fn wrap<W>(self, ctx: &IoContext, wrapper: W) -> Self::Output
        where W: FnOnce(&IoContext, Self::Handler)
    {
        wrapper(ctx, self)
    }
}

impl<T, F, R, E> Complete<R, E> for ArcHandler<T, F, R, E>
where
    T: AsIoContext + Send + Sync + 'static,
    F: FnOnce(Arc<T>, Result<R, E>)
        + Send
        + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    fn success(self, this: &mut ThreadIoContext, res: R) {
        let ArcHandler {
            data,
            handler,
            _marker,
        } = self;
        handler(data, Ok(res));
        this.decrease_outstanding_work();
    }

    fn failure(self, this: &mut ThreadIoContext, err: E) {
        let ArcHandler {
            data,
            handler,
            _marker,
        } = self;
        handler(data, Err(err));
        this.decrease_outstanding_work();
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
/// fn on_accept(soc: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
///   if let Ok((acc, ep)) = res {
///     println!("accepted {}", ep)
///   }
/// }
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = Arc::new(TcpListener::new(ctx, Tcp::v4()).unwrap());
/// soc.async_accept(wrap(on_accept, &soc));
/// ```
pub fn wrap<T, F, R, E>(handler: F, data: &Arc<T>) -> ArcHandler<T, F, R, E> {
    ArcHandler {
        data: data.clone(),
        handler: handler,
        _marker: PhantomData,
    }
}
