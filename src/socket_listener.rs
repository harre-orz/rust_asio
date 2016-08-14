use std::io;
use std::mem;
use std::marker::PhantomData;
use {IoObject, IoService, Protocol, IoControl, GetSocketOption, SetSocketOption, FromRawFd, Handler};
use backbone::{SOMAXCONN, RawFd, AsRawFd, IoActor, AsIoActor, socket, bind, listen,
               getsockname, ioctl, getsockopt, setsockopt, getnonblock, setnonblock};
use backbone::ops::{accept, async_accept, cancel_io};

struct AcceptHandler<P, F, S> {
    pro: P,
    handler: F,
    marker: PhantomData<S>,
}

impl<P, F, S> Handler<SocketListener<P>, (RawFd, P::Endpoint)> for AcceptHandler<P, F, S>
    where P: Protocol,
          F: Handler<SocketListener<P>, (S, P::Endpoint)>,
          S: FromRawFd<P>,
{
    fn callback(self, io: &IoService, soc: &SocketListener<P>, res: io::Result<(RawFd, P::Endpoint)>) {
        let AcceptHandler { pro, handler, marker:_ } = self;
        match res {
            Ok((fd, ep)) => handler.callback(io, soc, Ok((unsafe { S::from_raw_fd(io, pro, fd) }, ep))),
            Err(err)     => handler.callback(io, soc, Err(err))
        };
    }
}

pub struct SocketListener<P> {
    pro: P,
    io: IoActor,
}

impl<P: Protocol> SocketListener<P> {
    pub fn new<T: IoObject>(io: &T, pro: P) -> io::Result<SocketListener<P>> {
        let fd = try!(socket(&pro));
        Ok(unsafe { Self::from_raw_fd(io, pro, fd) })
    }

    pub fn accept<S: FromRawFd<P>>(&self) -> io::Result<(S, P::Endpoint)> {
        let (fd, ep) = try!(accept(self, unsafe { mem::uninitialized() }));
        Ok((unsafe { S::from_raw_fd(self, self.protocol(), fd) }, ep))
    }

    pub fn async_accept<S: FromRawFd<P>, F: Handler<Self, (S, P::Endpoint)>>(&self, handler: F) {
        let wrap = AcceptHandler {
            pro: self.protocol(),
            handler: handler,
            marker: PhantomData,
        };
        async_accept(self, unsafe { mem::uninitialized() }, wrap);
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    pub fn cancel(&self) {
        cancel_io(self)
    }

    pub fn listen(&self) -> io::Result<()> {
        listen(self, SOMAXCONN)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }

    pub fn io_control<T: IoControl>(&self, cmd: &mut T) -> io::Result<()> {
        ioctl(self, cmd)
    }

    pub fn get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    pub fn get_option<T: GetSocketOption<P>>(&self) -> io::Result<T> {
        getsockopt(self, &self.pro)
    }

    pub fn protocol(&self) -> P {
        self.pro.clone()
    }

    pub fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }

    pub fn set_option<T: SetSocketOption<P>>(&self, cmd: T) -> io::Result<()> {
        setsockopt(self, &self.pro, cmd)
    }
}

impl<P> IoObject for SocketListener<P> {
    fn io_service(&self) -> &IoService {
        self.io.io_service()
    }
}

impl<P: Protocol> FromRawFd<P> for SocketListener<P> {
    unsafe fn from_raw_fd<T: IoObject>(io: &T, pro: P, fd: RawFd) -> SocketListener<P> {
        SocketListener {
            pro: pro,
            io: IoActor::new(io, fd),
        }
    }
}

impl<P> AsRawFd for SocketListener<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.io.as_raw_fd()
    }
}

impl<P: Protocol> AsIoActor for SocketListener<P> {
    fn as_io_actor(&self) -> &IoActor {
        &self.io
    }
}
