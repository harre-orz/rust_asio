use std::io;
use std::sync::Arc;
use std::marker::PhantomData;
use {IoObject, IoService, Handler};

/// The binding Arc<T> handler.
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
    fn callback(self, _: &IoService, res: io::Result<R>) {
        let ArcHandler { owner, handler, marker:_ } = self;
        handler(owner, res)
    }
}

/// Provides a primitive handler to asynchronous operation.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use asio::{IoService, ArcHandler, bind};
/// use asio::ip::{Tcp, TcpSocket, TcpListener};
///
/// let io = &IoService::new();
/// let soc = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
/// soc.async_accept(bind(|soc, res| {
///   let _: Arc<TcpListener> = soc;
///
///   if let Ok((acc, ep)) = res {
///     let _: TcpSocket = acc;
///     println!("accepted {}", ep)
///   }
/// }, &soc));
/// ```
pub fn bind<T, F, R>(handler: F, owner: &Arc<T>) -> ArcHandler<T, F, R> {
    ArcHandler {
        owner: owner.clone(),
        handler: handler,
        marker: PhantomData,
    }
}
