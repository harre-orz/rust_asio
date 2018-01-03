use core::{IoContext, ThreadIoContext, Task, Yield};

pub struct NoYield;

impl Yield<()> for NoYield {
    fn yield_return(self, _: &IoContext) {}
}


pub trait Handler<R, E> : Send + 'static {
    type Output;

    #[doc(hidden)]
    type Perform : Handler<R, E> + Task;

    #[doc(hidden)]
    type Yield : Yield<Self::Output>;

    #[doc(hidden)]
    fn channel(self) -> (Self::Perform, Self::Yield);

    #[doc(hidden)]
    fn complete(self, this: &mut ThreadIoContext, res: Result<R, E>);

    #[doc(hidden)]
    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: R);

    #[doc(hidden)]
    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: E);
}

mod accept_op;
pub use self::accept_op::*;

mod connect_op;
pub use self::connect_op::*;

mod read_op;
pub use self::read_op::*;

mod write_op;
pub use self::write_op::*;
