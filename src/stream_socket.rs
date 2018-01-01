use ffi::*;
use core::*;
use prelude::*;
use buffers::{ConstBuffer, MutableBuffer};
use socket_base;

use std::io;
use std::marker::PhantomData;
use std::time::Duration;
use std::io::{Read, Write};


pub struct StreamSocket<P, M> {
    soc: Box<SocketImpl<P>>,
    _marker: PhantomData<M>
}


impl<P, M: Default + Send + 'static> StreamSocket<P, M>
    where P: Protocol,
{
    pub fn new(io: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro).map_err(error)?;
        Ok(StreamSocket {
            pro: pro,
            soc: SocketImpl::new(io, soc),
        })
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = socket_base::BytesReadable::default();
        ioctl(self, &mut bytes).map_err(error)?;
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep).map_err(error)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self).map_err(error)
     }

    pub fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
    {
        getsockopt(self).map_err(error)
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
        where C: IoControl
    {
        ioctl(self, cmd).map_err(error)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self).map_err(error)
    }

    pub fn set_socket_option<C>(&self, cmd: C) -> io::Result<()>
        where C: SetSocketOption<P>
    {
        setsockopt(self, cmd).map_err(error)
    }

    pub fn shutdown(&self, how: socket_base::Shutdown) -> io::Result<()> {
        shutdown(self, how as i32).map_err(error)
    }
}


impl<P> StreamSocket<P, socket_base::Sync>
    where P: Protocol
{
    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        connect(self, ep).map_err(error)
    }

    pub fn nonblocking_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags).map_err(error)
    }

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags).map_err(error)
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv_timeout(self, buf, flags, self.soc.mode.get_recv_timeout()).map_err(error)
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send_timeout(self, buf, flags, self.soc.mode.get_send_timeout()).map_err(error)
    }
}


impl<P> StreamSocket<P, socket_base::Async>
    where P: Protocol,
{
}

unsafe impl<P, M> AsIoContext for StreamSocket<P, M> {
    fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }
}


impl<P, M> AsRawFd for StreamSocket<P, M> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.as_raw_fd()
    }
}


impl<P, M> Socket<P> for StreamSocket<P, M>
    where P: Protocol,
          M: Send + 'static,
{
    fn protocol(&self) -> &P {
        &self.pro
    }
}


impl<P> Read for StreamSocket<P, socket_base::Sync>
    where P: Protocol,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read(self, buf).map_err(error)
    }
}


impl<P> Write for StreamSocket<P, socket_base::Sync>
    where P: Protocol,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write(self, buf).map_err(error)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
