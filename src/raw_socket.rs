use std::io;
use io_service::{IoObject, FromRawFd, IoService, IoActor, Handler, AsyncResult};
use traits::{Protocol, IoControl, GetSocketOption, SetSocketOption, Shutdown};
use fd_ops::*;
use socket_base::BytesReadable;

/// Provides a raw-oriented socket.
pub struct RawSocket<P: Protocol> {
    pro: P,
    act: IoActor,
}

impl<P: Protocol> RawSocket<P> {
    pub fn new(io: &IoService, pro: P) -> io::Result<RawSocket<P>> {
        let fd = try!(socket(&pro));
        Ok(unsafe { Self::from_raw_fd(io, pro, fd) })
    }

    pub fn async_connect<F>(&self, ep: &P:: Endpoint, handler: F) -> F::Output
        where F: Handler<()>,
    {
        let out = handler.async_result();
        let res = self.connect(ep);
        self.io_service().post(move |io| handler.callback(io, res));
        out.get(self.io_service())
    }

    pub fn async_receive<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize>,
    {
        async_recv(self, buf, flags, handler)
    }

    pub fn async_receive_from<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
        where F: Handler<(usize, P::Endpoint)>,
    {
        async_recvfrom(self, buf, flags, unsafe { self.pro.uninitialized() }, handler)
    }

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize>,
    {
        async_send(self, buf, flags, handler)
    }

    pub fn async_send_to<F>(&self, buf: &[u8], flags: i32, ep: P::Endpoint, handler: F) -> F::Output
        where F: Handler<usize>,
    {
        async_sendto(self, buf, flags, ep, handler)
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = BytesReadable::default();
        try!(self.io_control(&mut bytes));
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    pub fn cancel(&self) {
        cancel(self)
    }

    pub fn connect(&self, ep: &P:: Endpoint) -> io::Result<()> {
        connect(self, ep)
    }

    pub fn get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    pub fn get_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>,
    {
        getsockopt(self, &self.pro)
    }

    pub fn io_control<T>(&self, cmd: &mut T) -> io::Result<()>
        where T: IoControl,
    {
        ioctl(self, cmd)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self, unsafe { self.pro.uninitialized() })
    }

    pub fn protocol(&self) -> P {
        self.pro.clone()
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags)
    }

    pub fn receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        recvfrom(self, buf, flags, unsafe { self.pro.uninitialized() })
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self, unsafe { self.pro.uninitialized() })
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags)
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: P::Endpoint) -> io::Result<usize> {
        sendto(self, buf, flags, ep)
    }

    pub fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }

    pub fn set_option<C>(&self, cmd: C) -> io::Result<()>
        where C: SetSocketOption<P>,
    {
        setsockopt(self, &self.pro, cmd)
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        shutdown(self, how)
    }
}

impl<P: Protocol> IoObject for RawSocket<P> {
    fn io_service(&self) -> &IoService {
        self.act.io_service()
    }
}

impl<P: Protocol> FromRawFd<P> for RawSocket<P> {
    unsafe fn from_raw_fd(io: &IoService, pro: P, fd: RawFd) -> RawSocket<P> {
        RawSocket {
            pro: pro,
            act: IoActor::new(io, fd),
        }
    }
}

impl<P: Protocol> AsRawFd for RawSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.act.as_raw_fd()
    }
}

impl<P: Protocol> AsIoActor for RawSocket<P> {
    fn as_io_actor(&self) -> &IoActor {
        &self.act
    }
}
