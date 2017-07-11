use ffi::*;
use core::*;
use prelude::*;
use socket_base;

use std::io;
use std::marker::PhantomData;
use std::time::Duration;

pub struct StreamSocket<P, M> {
    soc: PairBox<SocketContext<P>>,
    _mode: PhantomData<M>,
}

impl<P> StreamSocket<P, socket_base::Rx>
    where P: Protocol,
{
    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = socket_base::BytesReadable::default();
        ioctl(self, &mut bytes).map_err(error)?;
        Ok(bytes.get())
    }

    pub fn get_non_blocking(&self) -> bool {
        !self.soc.recv_block
    }

    pub fn get_timeout(&self) -> Option<Duration> {
        self.soc.recv_timeout.clone()
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self).map_err(error)
    }

    pub fn receive(&mut self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        if self.soc.recv_block {
            readable(self, &self.soc.recv_timeout).map_err(error)?;
        }
        recv(self, buf, flags).map_err(error)
    }

    pub fn receive_from(&mut self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        if self.soc.recv_block {
            readable(self, &self.soc.recv_timeout).map_err(error)?;
        }
        recvfrom(self, buf, flags).map_err(error)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self).map_err(error)
    }

    pub fn set_non_blocking(&mut self, on: bool) {
        self.soc.recv_block = !on
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.soc.recv_timeout = timeout;
    }

    pub fn shutdown(self) -> io::Result<()> {
        shutdown(&self, SHUT_RD).map_err(error)
    }
}

impl<P> StreamSocket<P, socket_base::Tx>
    where P: Protocol,
{
    pub fn get_non_blocking(&self) -> bool {
        !self.soc.send_block
    }

    pub fn get_timeout(&self) -> Option<Duration> {
        self.soc.send_timeout.clone()
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self).map_err(error)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self).map_err(error)
    }

    pub fn send(&mut self, buf: &[u8], flags: i32) -> io::Result<usize> {
        if self.soc.send_block {
            let mut off = 0;
            while off < buf.len() {
                writable(self, &self.soc.send_timeout).map_err(error)?;
                off += send(self, &buf[off..], flags).map_err(error)?;
            }
            Ok(off)
        } else {
            send(self, buf, flags).map_err(error)
        }
    }

    pub fn send_to(&mut self, buf: &[u8], flags: i32, ep: P::Endpoint) -> io::Result<usize> {
        if self.soc.send_block {
            let mut off = 0;
            while off < buf.len() {
                writable(self, &self.soc.send_timeout).map_err(error)?;
                off += sendto(self, &buf[off..], flags, &ep).map_err(error)?;
            }
            Ok(off)
        } else {
            sendto(self, buf, flags, &ep).map_err(error)
        }
    }

    pub fn set_non_blocking(&mut self, on: bool) {
        self.soc.send_block = !on
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.soc.send_timeout = timeout;
    }

    pub fn shutdown(self) -> io::Result<()> {
        shutdown(&self, SHUT_WR).map_err(error)
    }
}

unsafe impl<P, M> AsIoContext for StreamSocket<P, M> {
    fn as_ctx(&self) -> &IoContext {
        &self.soc.ctx
    }
}

impl<P, M> AsRawFd for StreamSocket<P, M> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.fd
    }
}

impl<P, M> Socket<P> for StreamSocket<P, M>
    where P: Protocol,
          M: Send + 'static,
{
    fn protocol(&self) -> &P {
        &self.soc.pro
    }
}

impl<P> Tx<P> for StreamSocket<P, socket_base::Tx>
    where P: Protocol,
{
    fn from_ctx(soc: PairBox<SocketContext<P>>) -> Self {
        StreamSocket { soc: soc, _mode: PhantomData }
    }
}

impl<P> Rx<P> for StreamSocket<P, socket_base::Rx>
    where P: Protocol,
{
    fn from_ctx(soc: PairBox<SocketContext<P>>) -> Self {
        StreamSocket { soc: soc, _mode: PhantomData }
    }
}

impl<P> io::Read for StreamSocket<P, socket_base::Rx> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.soc.recv_block {
            readable(self, &self.soc.recv_timeout).map_err(error)?;
        }
        read(self, buf).map_err(error)
    }
}

impl<P> io::Write for StreamSocket<P, socket_base::Tx> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.soc.send_block {
            writable(self, &self.soc.send_timeout).map_err(error)?;
        }
        write(self, buf).map_err(error)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<P, M> SocketControl<P> for StreamSocket<P, M>
    where P: Protocol,
          M: Send + 'static,
{
    fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
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

    fn set_socket_option<C>(self, cmd: C) -> io::Result<Self>
        where C: SetSocketOption<P>
    {
        if self.soc.has_pair() {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        setsockopt(&self, cmd).map_err(error)?;
        Ok(self)
    }
}

impl<P> SocketControl<P> for (StreamSocket<P, socket_base::Tx>, StreamSocket<P, socket_base::Rx>)
    where P: Protocol,
{
    fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
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

    fn set_socket_option<C>(self, cmd: C) -> io::Result<Self>
        where C: SetSocketOption<P>
    {
        if !self.0.soc.is_pair(&self.1.soc) {
            return Err(io::Error::from_raw_os_error(EINVAL))
        }
        setsockopt(&self.0, cmd).map_err(error)?;
        Ok(self)
    }
}
