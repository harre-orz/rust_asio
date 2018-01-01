pub trait Handler<R, E> : Sized {
    type Output;

    fn result(self) -> Self::Output;
}


// use error::ErrCode;
// use core::{IoContext, ThreadIoContext, Upcast, FnOp};
//
// pub trait Sender<R, E, G: WrappedHandler<R, E>> : FnOp + Upcast<FnOp + Send> {
//     fn send(self: Box<Self>, &IoContext, Result<R, E>);
//
//     fn as_self(&self) -> &G;
//
//     fn as_mut_self(&mut self) -> &mut G;
// }
//
// impl<R, E, G> Into<Box<FnOp + Send>> for Box<Sender<R, E, G> + Send> {
//     fn into(self) -> Box<FnOp + Send> {
//         self.upcast()
//     }
// }
//
// pub type Operation<R, E, G> = Box<Sender<R, E, G> + Send>;
//
// pub trait Receiver<R> {
//     fn recv(self, &IoContext) -> R;
// }
//
// pub struct NullReceiver;
//
// impl Receiver<()> for NullReceiver {
//     fn recv(self, _: &IoContext) {
//     }
// }
//
// pub trait WrappedHandler<R, E> {
//     fn perform(&mut self, &IoContext, &mut ThreadIoContext, ErrCode, Operation<R, E, Self>);
// }
//
// pub trait Handler<R, E> : Sized {
//     type Output;
//
//     fn result(self, &IoContext, Result<R, E>) -> Self::Output;
//
//     #[doc(hidden)]
//     type Receiver : Receiver<Self::Output>;
//
//     #[doc(hidden)]
//     fn channel<G>(self, G) -> (Operation<R, E, G>, Self::Receiver)
//         where G: WrappedHandler<R, E> + Send + 'static;
// }
//
// mod wrap;
// pub use self::wrap::{wrap};
//
// mod strand;
// pub use self::strand::{Strand, StrandImmutable};
//
// #[cfg(feature = "context")] mod coroutine;
// #[cfg(feature = "context")] pub use self::coroutine::{Coroutine};
