use std::io;
use std::boxed::FnBox;
use std::sync::Arc;
use std::marker::PhantomData;
use super::{IoObject, IoService};

pub trait AsyncResult<R> {
    fn get(self, io: &IoService) -> R;
}

pub struct NoAsyncResult;

impl AsyncResult<()> for NoAsyncResult {
    fn get(self, _io: &IoService) {
    }
}

pub trait Handler<R> : Send + 'static {
    type Output;

    fn callback(self, io: &IoService, res: io::Result<R>);

    #[doc(hidden)]
    type AsyncResult : AsyncResult<Self::Output>;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult;
}


#[allow(dead_code)]
pub struct BoxedAsyncResult<R>(Box<FnBox(*const IoService) -> R>);

impl<R> BoxedAsyncResult<R> {
    #[allow(dead_code)]
    pub fn new<F>(func: F) -> BoxedAsyncResult<R>
        where F: FnOnce(&IoService) -> R + 'static
    {
        BoxedAsyncResult(Box::new(|io: *const IoService| func(unsafe { &*io })))
    }
}

impl<R> AsyncResult<R> for BoxedAsyncResult<R> {
    #[allow(dead_code)]
    fn get(self, io: &IoService) -> R {
        (self.0)(io)
    }
}


/// The binding Arc handler.
pub struct ArcHandler<T, F, R> {
    owner: Arc<T>,
    handler: F,
    _marker: PhantomData<R>,
}

impl<T, F, R> Handler<R> for ArcHandler<T, F, R>
    where T: IoObject + Send + Sync + 'static,
          F: FnOnce(Arc<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static,
{
    type Output = ();

    fn callback(self, _: &IoService, res: io::Result<R>) {
        let ArcHandler { owner, handler, _marker } = self;
        handler(owner, res)
    }

    #[doc(hidden)]
    type AsyncResult = NoAsyncResult;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult {
        NoAsyncResult
    }
}

/// Provides a `Arc` handler to asynchronous operation.
///
/// # Examples
///
/// ```
/// use std::io;
/// use std::sync::Arc;
/// use asyncio::{IoService, wrap};
/// use asyncio::ip::{Tcp, TcpSocket, TcpEndpoint, TcpListener};
///
/// fn on_accept(soc: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
///   if let Ok((acc, ep)) = res {
///     println!("accepted {}", ep)
///   }
/// }
///
/// let io = &IoService::new();
/// let soc = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
/// soc.async_accept(wrap(on_accept, &soc));
/// ```
pub fn wrap<T, F, R>(handler: F, owner: &Arc<T>) -> ArcHandler<T, F, R>
    where T: IoObject,
{
    ArcHandler {
        owner: owner.clone(),
        handler: handler,
        _marker: PhantomData,
    }
}
