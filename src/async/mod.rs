use ffi::SystemError;
use core::{ThreadIoContext, Perform};

pub trait Yield<T> {
    fn yield_return(self) -> T;
}


pub struct NoYield;

impl Yield<()> for NoYield {
    fn yield_return(self) {}
}


pub trait Complete<R, E>: Handler<R, E> {
    fn success(self, this: &mut ThreadIoContext, res: R);

    fn failure(self, this: &mut ThreadIoContext, err: E);
}


pub trait Handler<R, E>: Send + 'static {
    type Output;

    #[doc(hidden)]
    type Perform: Complete<R, E>;

    #[doc(hidden)]
    type Yield: Yield<Self::Output>;

    #[doc(hidden)]
    fn channel(self) -> (Self::Perform, Self::Yield);
}


pub trait AsyncSocketOp: Send + 'static {
    fn add_read_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn add_write_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    fn next_read_op(&mut self, this: &mut ThreadIoContext);

    fn next_write_op(&mut self, this: &mut ThreadIoContext);
}


pub trait AsyncWaitOp: Send + 'static {
    fn set_wait_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>);

    fn reset_wait_op(&mut self, this: &mut ThreadIoContext);
}


mod wait_op;
pub use self::wait_op::*;

mod accept_op;
pub use self::accept_op::*;

mod connect_op;
pub use self::connect_op::*;

mod read_op;
pub use self::read_op::*;

mod write_op;
pub use self::write_op::*;

mod wrap;
pub use self::wrap::*;

mod strand;
pub use self::strand::*;

mod coroutine;
pub use self::coroutine::*;
