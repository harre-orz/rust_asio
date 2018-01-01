use core::{IoContext, ThreadIoContext};

use std::sync::Arc;
use std::marker::PhantomData;

use errno::Errno;

pub trait Yield<T> {
    fn await(self, ctx: &IoContext) -> T;
}


pub struct NoYield;

impl Yield<()> for NoYield {
    fn await(self, _: &IoContext) {}
}


pub trait Handler<R, E> : Send + 'static {
    type Output;

    type Perform: Handler<R, E>;

    type Yield: Yield<Self::Output>;

    fn channel(self) -> (Self::Perform, Self::Yield);

    fn complete(self, this: &mut ThreadIoContext, res: Result<R, E>);

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: R);

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: E);
}


mod arc;


// mod read_op;
// pub use self::read_op::*;
