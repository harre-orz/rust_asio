use core::{Exec, ThreadIoContext};
use handler::Complete;

use std::marker::PhantomData;


pub struct ErrorHandler<F, R, E>(F, E, PhantomData<R>);

impl<F, R, E> ErrorHandler<F, R, E> {
    pub fn new(handler: F, err: E) -> Self {
        ErrorHandler(handler, err, PhantomData)
    }
}

impl<F, R, E> Exec for ErrorHandler<F, R, E>
where
    F: Complete<R, E>,
    R: Send + 'static,
    E: Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        let ErrorHandler(handler, err, _marker) = self;
        handler.failure(this, err)
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}
