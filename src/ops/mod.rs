use ffi::SystemError;
use core::{ThreadIoContext, Perform};

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

mod err_op;
pub use self::err_op::*;

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

mod stream_op;
pub use self::stream_op::*;
