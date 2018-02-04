use ffi::SystemError;
use core::{AsIoContext, Exec, Perform, ThreadIoContext};
use handler::Complete;
use std::marker::PhantomData;

pub trait AsyncReadOp: AsIoContext + Send + 'static {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn next_read_op(&self, this: &mut ThreadIoContext);
}

pub trait AsyncWriteOp: AsIoContext + Send + 'static {
    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn next_write_op(&self, this: &mut ThreadIoContext);
}

pub trait AsyncWaitOp: AsIoContext + Send + 'static {
    fn set_wait_op(&self, this: &mut ThreadIoContext, op: Box<Perform>);
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

mod accept_ops;
pub use self::accept_ops::*;

mod connect_ops;
pub use self::connect_ops::*;

mod read_ops;
pub use self::read_ops::*;

mod recv_ops;
pub use self::recv_ops::*;

mod recvfrom_ops;
pub use self::recvfrom_ops::*;

mod resolve_ops;
pub use self::resolve_ops::*;

mod send_ops;
pub use self::send_ops::*;

mod sendto_ops;
pub use self::sendto_ops::*;

mod stream_ops;
pub use self::stream_ops::*;

mod wait_ops;
pub use self::wait_ops::*;

mod write_ops;
pub use self::write_ops::*;

mod signal_wait;
pub use self::signal_wait::*;
