use super::{Handler, Yield, NoYield};
use core::{IoContext, ThreadIoContext};

use std::sync::Arc;
use std::marker::PhantomData;

use errno::Errno;


pub struct ArcHandler<T, F, R, E> {
    owner: Arc<T>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}

impl<T, F, R, E> Handler<R, E> for ArcHandler<T, F, R, E>
    where T: Send + Sync + 'static,
          F: FnOnce(Arc<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }

    fn complete(self, this: &mut ThreadIoContext, res: Result<R, E>) {
        let ArcHandler { owner, handler, _marker } = self;
    }

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: R) {
        self.complete(this, Ok(res))
    }

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: E) {
        self.complete(this, Err(err))
    }
}
