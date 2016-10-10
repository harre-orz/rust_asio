use std::io;
use io_service::{IoObject, FromRawFd, IoService, IoActor, Handler};
use traits::{Protocol, IoControl, GetSocketOption, SetSocketOption, Shutdown};
use stream::{Stream};
use fd_ops::*;
use socket_base::BytesReadable;

/// Provides a stream-oriented socket.
pub struct StreamSocket<P: Protocol> {
    pro: P,
    act: IoActor,
}

impl<P: Protocol> StreamSocket<P> {
    pub fn new(io: &IoService, pro: P) -> io::Result<StreamSocket<P>> {
        let fd = try!(socket(&pro));
        Ok(unsafe { Self::from_raw_fd(io, pro, fd) })
    }

    pub fn async_connect<F>(&self, ep: &P::Endpoint, handler: F) -> F::Output
        where F: Handler<()>,
    {
        async_connect(self, ep, handler)
    }

    pub fn async_receive<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize>,
    {
        async_recv(self, buf, flags, handler)
    }

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
        where F: Handler<usize>,
    {
        async_send(self, buf, flags, handler)
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

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
        where C: IoControl,
    {
        ioctl(self, cmd)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self, unsafe { self.pro.uninitialized() })
    }

    pub fn protocol(&self) -> &P {
        &self.pro
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self, unsafe { self.pro.uninitialized() })
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags)
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

impl<P: Protocol> Stream for StreamSocket<P> {
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize>,
    {
        async_read(self, buf, handler)
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize>,
    {
        async_write(self, buf, handler)
    }

    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        read(self, buf)
    }

    fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        write(self, buf)
    }
}

impl<P: Protocol> IoObject for StreamSocket<P> {
    fn io_service(&self) -> &IoService {
        self.act.io_service()
    }
}

impl<P: Protocol> FromRawFd<P> for StreamSocket<P> {
    unsafe fn from_raw_fd(io: &IoService, pro: P, fd: RawFd) -> StreamSocket<P> {
        StreamSocket {
            pro: pro,
            act: IoActor::new(io, fd),
        }
    }
}

impl<P: Protocol> AsRawFd for StreamSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.act.as_raw_fd()
    }
}

impl<P: Protocol> AsIoActor for StreamSocket<P> {
    fn as_io_actor(&self) -> &IoActor {
        &self.act
    }
}


#[test]
fn test_receive_error_of_non_connect() {
    use std::io;
    use std::sync::Arc;
    use {IoService, wrap};
    use ip::Tcp;

    let io = &IoService::new();
    let soc = Arc::new(StreamSocket::new(io, Tcp::v4()).unwrap());

    let mut buf = [0; 256];
    assert!(soc.receive(&mut buf, 0).is_err());

    fn handler(_: Arc<StreamSocket<Tcp>>, res: io::Result<usize>) {
        assert!(res.is_err());
    }
    soc.async_receive(&mut buf, 0, wrap(handler, &soc));

    io.run();
}

#[test]
fn test_send_error_of_non_connect() {
    use std::io;
    use std::sync::Arc;
    use {IoService, wrap};
    use ip::Tcp;

    let io = &IoService::new();
    let soc = Arc::new(StreamSocket::new(io, Tcp::v4()).unwrap());

    let mut buf = [0; 256];
    assert!(soc.send(&mut buf, 0).is_err());

    fn handler(_: Arc<StreamSocket<Tcp>>, res: io::Result<usize>) {
        assert!(res.is_err());
    }
    soc.async_send(&mut buf, 0, wrap(handler, &soc));

    io.run();
}
