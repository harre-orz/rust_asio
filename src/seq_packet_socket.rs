use std::io;
use std::mem;
use {IoObject, IoService, Strand, Shutdown, Protocol, NonBlocking, IoControl, GetSocketOption, SetSocketOption, ConstBuffer, MutableBuffer, SeqPacketSocket};
use backbone::IoActor;
use socket_base::*;
use ops;
use ops::async::*;

impl<P: Protocol> SeqPacketSocket<P> {
    pub unsafe fn async_connect<F, T>(&self, ep: &P::Endpoint, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
        async_connect(self, ep, callback, strand)
    }

    pub unsafe fn async_receive<F, T>(&self, buf: MutableBuffer, flags: i32, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        async_recv(self, buf, flags, callback, strand)
    }

    pub unsafe fn async_send<F, T>(&self, buf: ConstBuffer, flags: i32, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        async_send(self, buf, flags, callback, strand)
    }

    pub fn at_mark(&self) -> io::Result<bool> {
        ops::at_mark::<Self, P>(self)
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = BytesReadable::default();
        try!(self.io_control(&mut bytes));
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        ops::bind(self, ep)
    }

    pub fn cancel(&self) {
        cancel_io(self)
    }

    pub fn conenct(&self, ep: &P::Endpoint) -> io::Result<()> {
        syncd_connect(self, ep)
    }

    pub fn get_option<T: GetSocketOption<P>>(&self) -> io::Result<T> {
        ops::getsockopt(self, &self.pro)
    }

    pub fn io_control<T: IoControl<P>>(&self, cmd: &mut T) -> io::Result<()> {
        ops::ioctl(self, cmd)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        ops::getsockname(self, unsafe { mem::uninitialized() })
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        syncd_recv(self, buf, flags)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        ops::getpeername(self, unsafe { mem::uninitialized() })
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        syncd_send(self, buf, flags)
    }

    pub fn set_option<T: SetSocketOption<P>>(&self, cmd: T) -> io::Result<()> {
        ops::setsockopt(self, &self.pro, cmd)
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        ops::shutdown(self, how)
    }
}

impl<P: Protocol> IoObject for SeqPacketSocket<P> {
    fn io_service(&self) -> &IoService {
        self.actor.io_service()
    }
}

impl<P: Protocol> NonBlocking for SeqPacketSocket<P> {
    fn get_non_blocking(&self) -> bool {
        self.nonblock.get()
    }

    fn set_non_blocking(&self, on: bool) {
        self.nonblock.set(on)
    }
}

impl<P: Protocol> ops::AsRawFd for SeqPacketSocket<P> {
    fn as_raw_fd(&self) -> ops::RawFd {
        self.actor.as_raw_fd()
    }
}

impl<P: Protocol> AsIoActor for SeqPacketSocket<P> {
    fn as_io_actor(&self) -> &IoActor {
        &self.actor
    }
}
