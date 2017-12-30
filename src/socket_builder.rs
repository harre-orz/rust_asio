use ffi::*;
use core::*;
use prelude::*;

use std::io;
use std::marker::PhantomData;
use std::time::Duration;


pub struct DgramSocketBuilder<P, T, R> {
    soc: SocketContext<P>,
    _tag: PhantomData<(P, T, R)>,
}

impl<P, T, R> DgramSocketBuilder<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<DgramSocketBuilder<P, T, R>> {
        let fd = socket(&pro).map_err(error)?;
        Ok(DgramSocketBuilder {
            soc: SocketContext::new(ctx, pro, fd),
            _tag: PhantomData,
        })
    }

    pub fn bind(&mut self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep).map_err(error)
    }

    pub fn connect(self, ep: &P::Endpoint) -> io::Result<(T, R)> {
        connect(&self, ep).map_err(error)?;
        if self.soc.send_block {
            writable(&self, &self.soc.send_timeout).map_err(error)?;
        }
        self.open()
    }

    pub fn get_non_blocking(&self) -> bool {
        !self.soc.send_block
    }

    pub fn get_timeout(&self) -> Option<Duration> {
        self.soc.send_timeout.clone()
    }

    pub fn open(self) -> io::Result<(T, R)> {
        let (tx, rx) = PairBox::new(self.soc);
        Ok((T::from_ctx(tx), R::from_ctx(rx)))
    }

    pub fn set_non_blocking(&mut self, on: bool) {
        self.soc.send_block = !on;
        self.soc.recv_block = !on;
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.soc.send_timeout = timeout;
    }
}

unsafe impl<P, T, R> AsIoContext for DgramSocketBuilder<P, T, R> {
    fn as_ctx(&self) -> &IoContext {
        &self.soc.ctx
    }
}

impl<P, T, R> AsRawFd for DgramSocketBuilder<P, T, R> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.fd
    }
}

impl<P, T, R> Socket<P> for DgramSocketBuilder<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    fn protocol(&self) -> &P {
        &self.soc.pro
    }
}

impl<P, T, R> SocketControl<P> for DgramSocketBuilder<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
    {
        getsockopt(self).map_err(error)
    }

    fn io_control<C>(self, cmd: &mut C) -> io::Result<Self>
        where C: IoControl,
    {
        ioctl(&self, cmd).map_err(error)?;
        Ok(self)
    }

    fn set_socket_option<C>(self, cmd: C) -> io::Result<Self>
        where C: SetSocketOption<P>
    {
        setsockopt(&self, cmd).map_err(error)?;
        Ok(self)
    }
}




pub struct StreamSocketBuilder<P, T, R> {
    soc: SocketContext<P>,
    _tag: PhantomData<(P, T, R)>,
}

impl<P, T, R> StreamSocketBuilder<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<StreamSocketBuilder<P, T, R>> {
        let fd = socket(&pro).map_err(error)?;
        Ok(StreamSocketBuilder {
            soc: SocketContext::new(ctx, pro, fd),
            _tag: PhantomData,
        })
    }

    pub fn bind(&mut self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep).map_err(error)
    }

    pub fn connect(self, ep: &P::Endpoint) -> io::Result<(T, R)> {
        connect(&self, ep).map_err(error)?;
        if self.soc.send_block {
            writable(&self, &self.soc.send_timeout).map_err(error)?;
        }
        self.open()
    }

    pub fn get_non_blocking(&self) -> bool {
        !self.soc.send_block
    }

    pub fn get_timeout(&self) -> Option<Duration> {
        self.soc.send_timeout.clone()
    }

    pub fn open(self) -> io::Result<(T, R)> {
        let (tx, rx) = PairBox::new(self.soc);
        Ok((T::from_ctx(tx), R::from_ctx(rx)))
    }

    pub fn set_non_blocking(&mut self, on: bool) {
        self.soc.send_block = !on;
        self.soc.recv_block = !on;
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.soc.send_timeout = timeout;
    }
}

unsafe impl<P, T, R> AsIoContext for StreamSocketBuilder<P, T, R> {
    fn as_ctx(&self) -> &IoContext {
        &self.soc.ctx
    }
}

impl<P, T, R> AsRawFd for StreamSocketBuilder<P, T, R> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.fd
    }
}

impl<P, T, R> Socket<P> for StreamSocketBuilder<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    fn protocol(&self) -> &P {
        &self.soc.pro
    }
}

impl<P, T, R> SocketControl<P> for StreamSocketBuilder<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
    {
        getsockopt(self).map_err(error)
    }

    fn io_control<C>(self, cmd: &mut C) -> io::Result<Self>
        where C: IoControl,
    {
        ioctl(&self, cmd).map_err(error)?;
        Ok(self)
    }

    fn set_socket_option<C>(self, cmd: C) -> io::Result<Self>
        where C: SetSocketOption<P>
    {
        setsockopt(&self, cmd).map_err(error)?;
        Ok(self)
    }
}



pub struct AcceptSocketBuilder<P, T, R> {
    soc: SocketContext<P>,
    _tag: PhantomData<(P, T, R)>,
}

impl<P, T, R> AcceptSocketBuilder<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<AcceptSocketBuilder<P, T, R>> {
        let fd = socket(&pro).map_err(error)?;
        Ok(AcceptSocketBuilder {
            soc: SocketContext::new(ctx, pro, fd),
            _tag: PhantomData,
        })
    }

    pub fn bind(&mut self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep).map_err(error)
    }

    pub fn connect(self, ep: &P::Endpoint) -> io::Result<(T, R)> {
        connect(&self, ep).map_err(error)?;
        if self.soc.send_block {
            writable(&self, &self.soc.send_timeout).map_err(error)?;
        }
        self.open()
    }

    pub fn get_non_blocking(&self) -> bool {
        !self.soc.send_block
    }

    pub fn get_timeout(&self) -> Option<Duration> {
        self.soc.send_timeout.clone()
    }

    pub fn open(self) -> io::Result<(T, R)> {
        let (tx, rx) = PairBox::new(self.soc);
        Ok((T::from_ctx(tx), R::from_ctx(rx)))
    }

    pub fn set_non_blocking(&mut self, on: bool) {
        self.soc.send_block = !on;
        self.soc.recv_block = !on;
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.soc.send_timeout = timeout;
    }
}

unsafe impl<P, T, R> AsIoContext for AcceptSocketBuilder<P, T, R> {
    fn as_ctx(&self) -> &IoContext {
        &self.soc.ctx
    }
}

impl<P, T, R> AsRawFd for AcceptSocketBuilder<P, T, R> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.fd
    }
}

impl<P, T, R> Socket<P> for AcceptSocketBuilder<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    fn protocol(&self) -> &P {
        &self.soc.pro
    }
}

impl<P, T, R> SocketControl<P> for AcceptSocketBuilder<P, T, R>
    where P: Protocol,
          T: Tx<P>,
          R: Rx<P>,
{
    fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
    {
        getsockopt(self).map_err(error)
    }

    fn io_control<C>(self, cmd: &mut C) -> io::Result<Self>
        where C: IoControl,
    {
        ioctl(&self, cmd).map_err(error)?;
        Ok(self)
    }

    fn set_socket_option<C>(self, cmd: C) -> io::Result<Self>
        where C: SetSocketOption<P>
    {
        setsockopt(&self, cmd).map_err(error)?;
        Ok(self)
    }
}
