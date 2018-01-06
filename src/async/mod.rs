use core::{IoContext, AsIoContext, ThreadIoContext, Task};

pub trait Yield<T> {
    fn yield_return(self, ctx: &IoContext) -> T;
}


pub struct NoYield;

impl Yield<()> for NoYield {
    fn yield_return(self, _: &IoContext) {}
}

pub trait Complete<R, E> : Send + 'static {
}

pub trait Handler<R, E> : Send + 'static {
    type Output;

    #[doc(hidden)]
    type Perform : Handler<R, E>;

    #[doc(hidden)]
    type Yield : Yield<Self::Output>;

    #[doc(hidden)]
    fn channel(self) -> (Self::Perform, Self::Yield);

    fn complete(self, this: &mut ThreadIoContext, res: Result<R, E>);

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: R);

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: E);
}


mod wrap;
pub use self::wrap::*;

mod strand;
pub use self::strand::*;

mod coroutine;
pub use self::coroutine::*;

mod accept_op;
pub use self::accept_op::*;

mod connect_op;
pub use self::connect_op::*;

mod read_op;
pub use self::read_op::*;

mod write_op;
pub use self::write_op::*;
