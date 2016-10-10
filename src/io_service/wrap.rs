use std::io;
use std::sync::Arc;
use std::marker::PhantomData;
use super::{IoObject, IoService};
use async_result::{Handler, NullAsyncResult};

/// The binding Arc handler.
pub struct ArcHandler<T, F, R> {
    owner: Arc<T>,
    handler: F,
    marker: PhantomData<R>,
}

impl<T, F, R> Handler<R> for ArcHandler<T, F, R>
    where T: IoObject + Send + Sync + 'static,
          F: FnOnce(Arc<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static,
{
    type Output = ();

    #[doc(hidden)]
    type AsyncResult = NullAsyncResult;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult {
        NullAsyncResult
    }

    fn callback(self, _: &IoService, res: io::Result<R>) {
        let ArcHandler { owner, handler, marker:_ } = self;
        handler(owner, res)
    }
}

/// Provides a Arc<T> handler to asynchronous operation.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use asyncio::{IoService, wrap};
/// use asyncio::ip::{Tcp, TcpSocket, TcpListener};
///
/// let io = &IoService::new();
/// let soc = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
/// soc.async_accept(wrap(|soc, res| {
///   let _: Arc<TcpListener> = soc;
///
///   if let Ok((acc, ep)) = res {
///     let _: TcpSocket = acc;
///     println!("accepted {}", ep)
///   }
/// }, &soc));
/// ```
pub fn wrap<T, F, R>(handler: F, owner: &Arc<T>) -> ArcHandler<T, F, R> {
    ArcHandler {
        owner: owner.clone(),
        handler: handler,
        marker: PhantomData,
    }
}
