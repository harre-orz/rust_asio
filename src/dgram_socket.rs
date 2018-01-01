use prelude::*;
use ffi::*;
use core::{IoContext, AsIoContext, ThreadIoContext, SocketImpl};
//use async::{Yield, Handler, AsyncReceive, async_read_op};
use socket_base;

use std::io;
use std::marker::PhantomData;
use std::time::Duration;


pub struct DgramSocket<P, M> {
    soc: Box<SocketImpl<P>>,
    _marker: PhantomData<M>,
}

impl<P, M> DgramSocket<P, M>
    where P: Protocol,
          M: Send + 'static,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro).map_err(error)?;
        Ok(DgramSocket {
            soc: SocketImpl::new(ctx, soc, pro),
            _marker: PhantomData,
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

impl<P> DgramSocket<P, socket_base::Sync>
    where P: Protocol
{
    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        connect(self, ep).map_err(error)
    }

    pub fn nonblocking_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags).map_err(error)
    }

    pub fn nonblocking_receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        recvfrom(self, buf, flags).map_err(error)
    }

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags).map_err(error)
    }

    pub fn nonblocking_send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        sendto(self, buf, flags, ep).map_err(error)
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags).map_err(error)
    }

    pub fn receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        recvfrom(self, buf, flags).map_err(error)
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags).map_err(error)
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        sendto(self, buf, flags, ep).map_err(error)
    }
}

impl<P: Protocol> DgramSocket<P, socket_base::Async> {
    // fn async_receive<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
    //     where F: Handler<usize, io::Error>
    // {
    //     let (handler, coro) = handler.channel();
    //     self.as_ctx().do_dispatch(AsyncReceive {
    //         socket: &*self.soc,
    //         buffer: buf.as_ptr(),
    //         buflen: buf.len(),
    //         handler: handler,
    //         errcode: Default::default(),
    //     });
    //     coro.await(self.as_ctx())
    // }
}

unsafe impl<P, M> AsIoContext for DgramSocket<P, M> {
    fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }
}

impl<P, M> AsRawFd for DgramSocket<P, M> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc.as_raw_fd()
    }
}

impl<P, M> Socket<P> for DgramSocket<P, M>
    where P: Protocol,
          M: Send + 'static,
{
    fn protocol(&self) -> &P {
        self.soc.protocol()
    }
}
