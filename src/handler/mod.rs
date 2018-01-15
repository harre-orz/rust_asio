use core::ThreadIoContext;

pub trait Complete<R, E>: Handler<R, E> {
    fn success(self, this: &mut ThreadIoContext, res: R);

    fn failure(self, this: &mut ThreadIoContext, err: E);
}

pub trait Yield<T> {
    fn yield_return(self) -> T;
}

pub struct NoYield;

impl Yield<()> for NoYield {
    fn yield_return(self) {}
}

pub trait Handler<R, E>: Send + 'static {
    type Output;

    #[doc(hidden)]
    type Caller: Complete<R, E>;

    #[doc(hidden)]
    type Callee: Yield<Self::Output>;

    #[doc(hidden)]
    fn channel(self) -> (Self::Caller, Self::Callee);
}

mod wrap;
pub use self::wrap::*;

mod strand;
pub use self::strand::*;

mod coroutine;
pub use self::coroutine::*;
