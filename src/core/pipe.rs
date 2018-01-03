use super::SocketImpl;
use ffi::{SystemError, pipe, write};
use std::io;

pub struct PipeIntr {
    pipe: Box<(SocketImpl, SocketImpl)>,
}

impl PipeIntr {
    pub fn new() -> io::Result<Self> {
        let (rfd, wfd) = pipe()?;
        PipeIntr {
            pipe: box ((SocketImpl::intr(rfd), SocketImpl::intr(wfd)))
        }
    }
}
