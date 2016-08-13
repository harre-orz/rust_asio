use std::io;
use std::sync::Arc;
use std::marker::PhantomData;
use {IoService, Handler};

pub struct ArcHandler<T, F, R> {
    owner: Arc<T>,
    handler: F,
    marker: PhantomData<R>,
}

impl<T, F, A, R> Handler<A, R> for ArcHandler<T, F, R>
    where T: Send + Sync + 'static,
          F: FnOnce(Arc<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static,
{
    fn callback(self, _: &IoService, _: &A, res: io::Result<R>) {
        let ArcHandler { owner, handler, marker:_ } = self;
        handler(owner, res)
    }
}

pub fn bind<T, F, R>(handler: F, owner: &Arc<T>) -> ArcHandler<T, F, R> {
    ArcHandler {
        owner: owner.clone(),
        handler: handler,
        marker: PhantomData,
    }
}
