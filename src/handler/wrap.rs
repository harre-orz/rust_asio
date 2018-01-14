use core::ThreadIoContext;
use handler::{Complete, Handler, NoYield};

use std::marker::PhantomData;
use std::sync::Arc;

pub struct ArcHandler<T, F, R, E> {
    data: Arc<T>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}

impl<T, F, R, E> Handler<R, E> for ArcHandler<T, F, R, E>
where
    T: Send + Sync + 'static,
    F: FnOnce(Arc<T>, Result<R, E>) + Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    type Output = ();

    #[doc(hidden)]
    type Perform = Self;

    #[doc(hidden)]
    type Yield = NoYield;

    #[doc(hidden)]
    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<T, F, R, E> Complete<R, E> for ArcHandler<T, F, R, E>
where
    T: Send + Sync + 'static,
    F: FnOnce(Arc<T>, Result<R, E>) + Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    fn success(self, _: &mut ThreadIoContext, res: R) {
        let ArcHandler {
            data,
            handler,
            _marker,
        } = self;
        handler(data, Ok(res))
    }

    fn failure(self, _: &mut ThreadIoContext, err: E) {
        let ArcHandler {
            data,
            handler,
            _marker,
        } = self;
        handler(data, Err(err))
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
