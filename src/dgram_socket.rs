use prelude::*;
use ffi::*;
use core::{IoContext, AsIoContext, ThreadIoContext, SocketImpl};
use socket_base;

use std::io;
use std::marker::PhantomData;


pub struct DgramSocket<P, M> {
    soc: Box<SocketImpl<P>>,
    _marker: PhantomData<M>,
}

impl<P, M> DgramSocket<P, M>
    where P: Protocol,
          M: Send + 'static,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro)?;
        Ok(unsafe { Self::from_raw_fd(ctx, pro, soc) })
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = socket_base::BytesReadable::default();
        ioctl(self, &mut bytes)?;
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        bind(self, ep).map_err(From::from)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        getsockname(self).map_err(From::from)
     }

    pub fn get_socket_option<C>(&self) -> io::Result<C>
        where C: GetSocketOption<P>
    {
        getsockopt(self).map_err(From::from)
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
        where C: IoControl
    {
        ioctl(self, cmd).map_err(From::from)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        getpeername(self).map_err(From::from)
    }

    pub fn set_socket_option<C>(&self, cmd: C) -> io::Result<()>
        where C: SetSocketOption<P>
    {
        setsockopt(self, cmd).map_err(From::from)
    }

    pub fn shutdown(&self, how: socket_base::Shutdown) -> io::Result<()> {
        shutdown(self, how).map_err(From::from)
    }
}

impl<P> DgramSocket<P, socket_base::Sync>
    where P: Protocol
{
    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        if self.as_ctx().stopped() {
            return Err(OPERATION_CANCELED.into())
        }

        match connect(self, ep) {
            Ok(_) => Ok(()),
            Err(IN_PROGRESS) =>
                match writable(self, &Timeout::default()) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err.into()),
                },
            Err(err) => Err(err.into()),
        }
    }

    pub fn nonblocking_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            recv(self, buf, flags).map_err(From::from)
        }
    }

    pub fn nonblocking_receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            unsafe {
                let mut ep = self.protocol().uninitialized();
                ep.resize(0);
                return Ok((0, ep))
            }
        } else {
            recvfrom(self, buf, flags).map_err(From::from)
        }
    }

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            send(self, buf, flags).map_err(From::from)
        }
    }

    pub fn nonblocking_send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        if self.as_ctx().stopped() {
            Err(OPERATION_CANCELED.into())
        } else if buf.is_empty() {
            Ok(0)
        } else {
            sendto(self, buf, flags, ep).map_err(From::from)
        }
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0)
        }

        while !self.as_ctx().stopped() {
            match recv(self, buf, flags) {
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) | Err(WOULD_BLOCK) =>
                    if let Err(err) = readable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        if buf.is_empty() {
            unsafe {
                let mut ep = self.protocol().uninitialized();
                ep.resize(0);
                return Ok((0, ep))
            }
        }

        while !self.as_ctx().stopped() {
            match recvfrom(self, buf, flags) {
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) | Err(WOULD_BLOCK) =>
                    if let Err(err) = readable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0)
        }

        while !self.as_ctx().stopped() {
            match send(self, buf, flags) {
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) | Err(WOULD_BLOCK) =>
                    if let Err(err) = writable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0)
        }

        while !self.as_ctx().stopped() {
            match sendto(self, buf, flags, ep) {
                Ok(len) => return Ok(len),
                Err(INTERRUPTED) | Err(WOULD_BLOCK) =>
                    if let Err(err) = writable(self, &Timeout::default()) {
                        return Err(err.into())
                    },
                Err(err) => return Err(err.into()),
            }
        }
        Err(OPERATION_CANCELED.into())
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

    unsafe fn from_raw_fd(ctx: &IoContext, pro: P, soc: RawFd) -> Self {
        DgramSocket {
            soc: SocketImpl::new(ctx, pro, soc),
            _marker: PhantomData,
        }
    }
}
