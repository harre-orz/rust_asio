use ffi::*;
use core::*;
use prelude::*;
use socket_base;

use std::io;
use std::marker::PhantomData;

pub struct DgramSocket<P, M> {
    soc: PairBox<SocketContext<P>>,
    _mode: PhantomData<M>,
}

impl<P> DgramSocket<P, socket_base::Rx>
    where P: Protocol,
{
    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = socket_base::BytesReadable::default();
        ioctl(self, &mut bytes).map_err(error)?;
        Ok(bytes.get())
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self).map_err(error)
    }

    pub fn receive(&mut self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags).map_err(error)
    }

    pub fn receive_from(&mut self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        recvfrom(self, buf, flags).map_err(error)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self).map_err(error)
    }

    pub fn shutdown(self) -> io::Result<()> {
        shutdown(&self, SHUT_RD).map_err(error)
    }
}

impl<P> DgramSocket<P, socket_base::Tx>
    where P: Protocol,
{
    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self).map_err(error)
    }

    pub fn shutdown(self) -> io::Result<()> {
        shutdown(&self, SHUT_WR).map_err(error)
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags).map_err(error)
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: P::Endpoint) -> io::Result<usize> {
        sendto(self, buf, flags, &ep).map_err(error)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self).map_err(error)
    }
}

unsafe impl<P, M> AsIoContext for DgramSocket<P, M> {
    fn as_ctx(&self) -> &IoContext {
        &self.soc.ctx
    }
}

impl<P, M> AsRawFd for DgramSocket<P, M> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.fd
    }
}

impl<P, M> Socket<P> for DgramSocket<P, M>
    where P: Protocol,
          M: Send + 'static,
{
    fn protocol(&self) -> &P {
        &self.soc.pro
    }
}

impl<P> Tx<P> for DgramSocket<P, socket_base::Tx>
    where P: Protocol,
{
    fn from_ctx(soc: PairBox<SocketContext<P>>) -> Self {
        DgramSocket { soc: soc, _mode: PhantomData }
    }
}

impl<P> Rx<P> for DgramSocket<P, socket_base::Rx>
    where P: Protocol,
{
    fn from_ctx(soc: PairBox<SocketContext<P>>) -> Self {
        DgramSocket { soc: soc, _mode: PhantomData }
    }
}

impl<P, M> SocketControl<P> for DgramSocket<P, M>
    where P: Protocol,
          M: Send + 'static,
{
    fn get_non_blocking(&self) -> io::Result<bool> {
        if self.soc.has_pair() {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        self.soc.getnonblock()
    }

    fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>,
    {
        if self.soc.has_pair() {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        getsockopt(self).map_err(error)
    }

    fn io_control<C>(self, cmd: &mut C) -> io::Result<Self>
        where C: IoControl,
    {
        if self.soc.has_pair() {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        ioctl(&self, cmd).map_err(error)?;
        Ok(self)
    }

    fn set_non_blocking(self, on: bool) -> io::Result<Self> {
        if self.soc.has_pair() {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        self.soc.setnonblock(on)?;
        Ok(self)
    }

    fn set_socket_option<C>(self, cmd: C) -> io::Result<Self>
        where C: SetSocketOption<P>,
    {
        if self.soc.has_pair() {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        setsockopt(&self, cmd).map_err(error)?;
        Ok(self)
    }
}

impl<P> SocketControl<P> for (DgramSocket<P, socket_base::Tx>, DgramSocket<P, socket_base::Rx>)
    where P: Protocol,
{
    fn get_non_blocking(&self) -> io::Result<bool> {
        if !self.0.soc.is_pair(&self.1.soc) {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        self.0.soc.getnonblock()
    }

    fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>,
    {
        if !self.0.soc.is_pair(&self.1.soc) {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        getsockopt(&self.0).map_err(error)
    }

    fn io_control<C>(self, cmd: &mut C) -> io::Result<Self>
        where C: IoControl,
    {
        if !self.0.soc.is_pair(&self.1.soc) {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        ioctl(&self.0, cmd).map_err(error)?;
        Ok(self)
    }

    fn set_non_blocking(self, on: bool) -> io::Result<Self> {
        if !self.0.soc.is_pair(&self.1.soc) {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        self.0.soc.setnonblock(on)?;
        Ok(self)
    }

    fn set_socket_option<C>(self, cmd: C) -> io::Result<Self>
        where C: SetSocketOption<P>,
    {
        if !self.0.soc.is_pair(&self.1.soc) {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        setsockopt(&self.0, cmd).map_err(error)?;
        Ok(self)
    }
}
