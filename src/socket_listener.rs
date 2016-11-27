use std::io;
use std::marker::PhantomData;
use error::ErrCode;
use io_service::{IoObject, FromRawFd, IoService, IoActor, Callback, Handler};
use traits::{Protocol, IoControl, GetSocketOption, SetSocketOption};
use fd_ops::*;

const SOMAXCONN: u32 = 126;

struct AcceptHandler<P, F, S> {
    pro: P,
    handler: F,
    _marker: PhantomData<S>,
}

impl<P, F, S> Handler<(RawFd, P::Endpoint)> for AcceptHandler<P, F, S>
    where P: Protocol,
          F: Handler<(S, P::Endpoint)>,
          S: FromRawFd<P>,
{
    type Output = F::Output;

    fn callback(self, io: &IoService, res: io::Result<(RawFd, P::Endpoint)>) {
        let AcceptHandler { pro, handler, _marker } = self;
        match res {
            Ok((fd, ep)) => handler.callback(io, Ok((unsafe { S::from_raw_fd(io, pro, fd) }, ep))),
            Err(err)     => handler.callback(io, Err(err))
        };
    }

    fn wrap<G>(self, callback: G) -> Callback
        where G: FnOnce(&IoService, ErrCode, Self) + Send + 'static,
    {
        let AcceptHandler { pro, handler, _marker } = self;
        handler.wrap(move |io, ec, handler| {
            callback(io, ec, AcceptHandler {
                pro: pro,
                handler: handler,
                _marker: _marker,
            })
        })
    }

    type AsyncResult = F::AsyncResult;

    fn async_result(&self) -> Self::AsyncResult {
        self.handler.async_result()
    }
}

/// Provides an ability to accept new connections.
pub struct SocketListener<P: Protocol> {
    pro: P,
    act: IoActor,
}

impl<P: Protocol> SocketListener<P> {
    pub fn new(io: &IoService, pro: P) -> io::Result<SocketListener<P>> {
        let fd = try!(socket(&pro));
        Ok(unsafe { Self::from_raw_fd(io, pro, fd) })
    }

    pub fn accept<S>(&self) -> io::Result<(S, P::Endpoint)>
        where S: FromRawFd<P>,
    {
        let (fd, ep) = try!(accept(self, unsafe { self.pro.uninitialized() }));
        Ok((unsafe { S::from_raw_fd(self.io_service(), self.protocol(), fd) }, ep))
    }

    pub fn async_accept<S, F>(&self, handler: F) -> F::Output
        where S: FromRawFd<P>,
              F: Handler<(S, P::Endpoint)>,
    {
        let handler = AcceptHandler {
            pro: self.protocol(),
            handler: handler,
            _marker: PhantomData,
        };
        async_accept(self, unsafe { self.pro.uninitialized() }, handler)
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    pub fn cancel(&self) {
        cancel(self)
    }

    pub fn listen(&self) -> io::Result<()> {
        listen(self, SOMAXCONN)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self, unsafe { self.pro.uninitialized() })
    }

    pub fn io_control<T>(&self, cmd: &mut T) -> io::Result<()>
        where T: IoControl,
    {
        ioctl(self, cmd)
    }

    pub fn get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    pub fn get_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>,
    {
        getsockopt(self, &self.pro)
    }

    pub fn protocol(&self) -> P {
        self.pro.clone()
    }

    pub fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }

    pub fn set_option<C>(&self, cmd: C) -> io::Result<()>
        where C: SetSocketOption<P>,
    {
        setsockopt(self, &self.pro, cmd)
    }
}

unsafe impl<P: Protocol> IoObject for SocketListener<P> {
    fn io_service(&self) -> &IoService {
        self.act.io_service()
    }
}

impl<P: Protocol> FromRawFd<P> for SocketListener<P> {
    unsafe fn from_raw_fd(io: &IoService, pro: P, fd: RawFd) -> SocketListener<P> {
        SocketListener {
            pro: pro,
            act: IoActor::new(io, fd),
        }
    }
}

impl<P: Protocol> AsRawFd for SocketListener<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.act.as_raw_fd()
    }
}

impl<P: Protocol> AsIoActor for SocketListener<P> {
    fn as_io_actor(&self) -> &IoActor {
        &self.act
    }
}
