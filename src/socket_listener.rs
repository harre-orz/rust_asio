use std::io;
use std::mem;
use {IoObject, IoService, Protocol, NonBlocking, IoControl, GetSocketOption, SetSocketOption, SocketListener};
use backbone::IoActor;
use ops;
use ops::async::*;

impl<P: Protocol> SocketListener<P> {
    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        ops::bind(self, ep)
    }

    pub fn cancel(&self) {
        cancel_io(self)
    }

    pub fn io_control<T: IoControl<Self>>(&self, cmd: &mut T) -> io::Result<()> {
        try!(ops::ioctl(self, cmd));
        Ok(())
    }

    pub fn listen(&self) -> io::Result<()> {
        ops::listen(self, ops::SOMAXCONN)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(try!(ops::getsockname(self, unsafe { mem::uninitialized() })))
    }

    pub fn get_option<T: GetSocketOption<Self>>(&self) -> io::Result<T> {
        ops::getsockopt(self)
    }

    pub fn set_option<T: SetSocketOption<Self>>(&self, cmd: &T) -> io::Result<()> {
        ops::setsockopt(self, cmd)
    }
}

impl<P: Protocol> IoObject for SocketListener<P> {
    fn io_service(&self) -> &IoService {
        self.actor.io_service()
    }
}

impl<P: Protocol> NonBlocking for SocketListener<P> {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl<P: Protocol> ops::AsRawFd for SocketListener<P> {
    fn as_raw_fd(&self) -> ops::RawFd {
        self.actor.as_raw_fd()
    }
}

impl<P: Protocol> AsIoActor for SocketListener<P> {
    fn as_io_actor(&self) -> &IoActor {
        &self.actor
    }
}
