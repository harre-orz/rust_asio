pub use std::os::unix::io::{RawFd, AsRawFd};
pub use libc::{sockaddr};
use error::ECANCELED;
use io_service::{IoObject, IoActor};

pub trait AsIoActor : IoObject + AsRawFd + 'static {
    fn as_io_actor(&self) -> &IoActor;
}

pub fn cancel<T>(fd: &T)
    where T: AsIoActor,
{
    let io = fd.io_service();

    for handler in fd.as_io_actor().del_input() {
        io.post(|io| handler(io, ECANCELED));
    }

    for handler in fd.as_io_actor().del_output() {
        io.post(|io| handler(io, ECANCELED));
    }
}

mod unix;
pub use self::unix::*;
