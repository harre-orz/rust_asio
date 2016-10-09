use std::io;
use std::boxed::FnBox;
use IoService;

pub trait AsyncResult<R> {
    fn result(self, io: &IoService) -> R;
}

pub trait Handler<R> : Send + 'static {
    type Output;

    type AsyncResult : AsyncResult<Self::Output>;

    fn async_result(&self) -> Self::AsyncResult;

    fn callback(self, io: &IoService, res: io::Result<R>);
}

pub struct NullAsyncResult;

impl AsyncResult<()> for NullAsyncResult {
    fn result(self, _io: &IoService) {
    }
}

pub struct BoxedAsyncResult<R>(Box<FnBox(*const IoService) -> R>);

impl<R> BoxedAsyncResult<R> {
    pub fn new<F>(func: F) -> BoxedAsyncResult<R>
        where F: FnOnce(&IoService) -> R + 'static
    {
        BoxedAsyncResult(Box::new(|io: *const IoService| func(unsafe { &*io })))
    }
}

impl<R> AsyncResult<R> for BoxedAsyncResult<R> {
    fn result(self, io: &IoService) -> R {
        (self.0)(io)
    }
}
