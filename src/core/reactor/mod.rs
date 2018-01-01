pub struct SocketImpl<M> {
    io: IoContext,
    fd: RawFd,
    pub mode: M,
}


impl<M: Default> SocketImpl<M> {
    pub fn new(io: &IoContext, fd: RawFd) -> Self {
        SocketImpl {
            io: io.clone(),
            fd: fd,
            mode: M::default(),
        }
    }
}

unsafe impl<M> AsIoContext for SocketImpl<M> {
    fn as_ctx(&self) -> &IoContext {
        &self.io
    }
}

impl<M> AsRawFd for SocketImpl<M> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}
