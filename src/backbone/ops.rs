use std::io;
use std::slice;
use libc;
use libc::{EINTR, EAGAIN, EINPROGRESS, c_void, sockaddr, socklen_t, ssize_t};
use {IoService, Endpoint, Handler};
use super::{ReactState, RawFd, AsRawFd, AsIoActor, AsWaitActor, Expiry, errno};

fn eof() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "End of File")
}

fn write_zero() -> io::Error {
    io::Error::new(io::ErrorKind::WriteZero, "Write Zero")
}

fn stopped() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Stopped")
}

fn canceled() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Operation Canceled")
}

fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused")
}

pub struct UnsafeRefCell<T> {
    ptr: *const T,
}

impl<T> UnsafeRefCell<T> {
    pub fn new(t: &T) -> UnsafeRefCell<T> {
        UnsafeRefCell { ptr: t }
    }

    pub unsafe fn as_ref(&self) -> &T {
        &*self.ptr
    }
}

unsafe impl<T> Send for UnsafeRefCell<T> {}

pub struct UnsafeSliceCell<T> {
    ptr: *const T,
    len: usize,
}

impl<T> UnsafeSliceCell<T> {
    pub fn new(t: &[T]) -> UnsafeSliceCell<T> {
        UnsafeSliceCell {
            ptr: t.as_ptr(),
            len: t.len(),
        }
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        slice::from_raw_parts(self.ptr, self.len)
    }

    pub unsafe fn as_slice_mut(&self) -> &mut [T] {
        slice::from_raw_parts_mut(self.ptr as *mut T, self.len)
    }
}

unsafe impl<T> Send for UnsafeSliceCell<T> {}

pub fn connect<T: AsIoActor, E: Endpoint>(fd: &T, ep: &E) -> io::Result<()> {
    if let Some(handler) = fd.as_io_actor().unset_output() {
        handler(fd.io_service(), ReactState::Canceled);
    }

    while !fd.io_service().stopped() {
        if unsafe { libc::connect(
            fd.as_raw_fd(),
            ep.as_sockaddr() as *const _ as *const sockaddr,
            ep.size() as socklen_t
        ) } == 0 { return Ok(()); }
        let ec = errno();
        if ec != EINTR {
            return Err(io::Error::from_raw_os_error(ec));
        }
    }

    Err(stopped())
}

pub fn async_connect<T: AsIoActor, E: Endpoint, F: Handler<T, ()>>(fd: &T, ep: &E, handler: F) {
    let io = fd.io_service();
    let fd_ptr = UnsafeRefCell::new(fd);

    let mode = fd.get_non_blocking().unwrap();
    fd.set_non_blocking(true).unwrap();
    while !fd.io_service().stopped() {
        if unsafe { libc::connect(
            fd.as_raw_fd(),
            ep.as_sockaddr() as *const _ as *const sockaddr,
            ep.size() as socklen_t
        ) } == 0 {
            if let Some(handler) = fd.as_io_actor().unset_output() {
                io.post(move |io| handler(io, ReactState::Canceled));
            }
            fd.set_non_blocking(mode).unwrap();
            io.post(move |io| handler.callback(io, unsafe { fd_ptr.as_ref() }, Ok(())));
            return;
        }

        let ec = errno();
        if ec == EINPROGRESS {
            fd.as_io_actor().set_output(Box::new(move |io: *const IoService, st: ReactState| {
                let io = unsafe { &*io };
                let fd = unsafe { fd_ptr.as_ref() };
                fd.as_io_actor().ready_output();
                fd.set_non_blocking(mode).unwrap();
                handler.callback(io, fd, match st {
                    ReactState::Ready
                        => Ok(()),
                    ReactState::Canceled
                        => Err(canceled()),
                    ReactState::Errored
                        => Err(refused()),
                });
            }));
            return;
        }
        if ec != EINTR {
            io.post(move |io| handler.callback(io, unsafe { fd_ptr.as_ref() }, Err(io::Error::from_raw_os_error(ec))));
            return;
        }
    }

    io.post(move |io| handler.callback(io, unsafe { fd_ptr.as_ref() }, Err(stopped())));
}

pub fn accept<T: AsIoActor, E: Endpoint>(fd: &T, mut ep: E) -> io::Result<(RawFd, E)> {
    if let Some(handler) = fd.as_io_actor().unset_input() {
        handler(fd.io_service(), ReactState::Canceled);
    }

    let mut socklen = ep.capacity() as socklen_t;
    while !fd.io_service().stopped() {
        let acc = unsafe { libc::accept(
            fd.as_raw_fd(),
            ep.as_mut_sockaddr() as *mut _ as *mut sockaddr,
            &mut socklen
        ) };
        if acc >= 0 {
            unsafe { ep.resize(socklen as usize); }
            return Ok((acc, ep));
        }
        let ec = errno();
        if ec != EINTR {
            return Err(io::Error::from_raw_os_error(ec));
        }
    }

    Err(stopped())
}

pub fn async_accept<T: AsIoActor, E: Endpoint, F: Handler<T, (RawFd, E)>>(fd: &T, mut ep: E, handler: F) {
    let fd_ptr = UnsafeRefCell::new(fd);

    fd.as_io_actor().set_input(Box::new(move |io: *const IoService, st: ReactState| {
        let io = unsafe { &*io };
        let fd = unsafe { fd_ptr.as_ref() };

        match st {
            ReactState::Ready => {
                if let Some(new_handler) = fd.as_io_actor().unset_input() {
                    handler.callback(io, fd, Err(canceled()));
                    new_handler(io, ReactState::Ready);
                    return;
                }

                let mode = fd.get_non_blocking().unwrap();
                fd.set_non_blocking(true).unwrap();

                let mut socklen = ep.capacity() as socklen_t;
                while !io.stopped() {
                    let acc = unsafe { libc::accept(
                        fd.as_raw_fd(),
                        ep.as_mut_sockaddr() as *mut _ as *mut sockaddr,
                        &mut socklen
                    ) };
                    if acc >= 0 {
                        unsafe { ep.resize(socklen as usize); }
                        fd.as_io_actor().ready_input();
                        fd.set_non_blocking(mode).unwrap();
                        handler.callback(io, fd, Ok((acc, ep)));
                        return;
                    }
                    let ec = errno();
                    if ec == EAGAIN {
                        fd.set_non_blocking(mode).unwrap();
                        async_accept(fd, ep, handler);
                        return;
                    }
                    if ec != EINTR {
                        fd.as_io_actor().ready_input();
                        fd.set_non_blocking(mode).unwrap();
                        handler.callback(io, fd, Err(io::Error::from_raw_os_error(ec)));
                        return;
                    }
                }
                fd.as_io_actor().ready_input();
                handler.callback(io, fd, Err(stopped()));
            },
            ReactState::Canceled => {
                handler.callback(io, fd, Err(canceled()));
            },
            ReactState::Errored
                => unreachable!(),
        }
    }));
}

trait Reader : Send + 'static{
    type Output;

    unsafe fn read<T: AsRawFd>(&mut self, fd: &T, buf: &mut [u8]) -> ssize_t;

    fn ok(self, len: ssize_t) -> Self::Output;
}

fn read_detail<T:AsIoActor, R: Reader>(fd: &T, buf: &mut [u8], mut reader: R) -> io::Result<R::Output> {
    if let Some(handler) = fd.as_io_actor().unset_input() {
        handler(fd.io_service(), ReactState::Canceled);
    }

    while !fd.io_service().stopped() {
        let len = unsafe { reader.read(fd, buf) };
        if len > 0 {
            return Ok(reader.ok(len));
        }
        if len == 0 {
            return Err(eof());
        }
        let ec = errno();
        if ec != EINTR {
            return Err(io::Error::from_raw_os_error(ec));
        }
    }

    Err(stopped())
}

fn async_read_detail<T: AsIoActor, R: Reader, F: Handler<T, R::Output>>(fd: &T, buf: &mut [u8], mut reader: R, handler: F) {
    let fd_ptr = UnsafeRefCell::new(fd);
    let buf_ptr = UnsafeSliceCell::new(buf);

    fd.as_io_actor().set_input(Box::new(move |io: *const IoService, st: ReactState| {
        let io = unsafe { &*io };
        let fd = unsafe { fd_ptr.as_ref() };

        match st {
            ReactState::Ready => {
                let buf = unsafe { buf_ptr.as_slice_mut() };

                if let Some(new_handler) = fd.as_io_actor().unset_input() {
                    io.post(|io| new_handler(io, ReactState::Ready));
                    handler.callback(io, fd, Err(canceled()));
                    return;
                }

                let mode = fd.get_non_blocking().unwrap();
                fd.set_non_blocking(true).unwrap();

                while !io.stopped() {
                    let len = unsafe { reader.read(fd, buf) };
                    if len > 0 {
                        fd.as_io_actor().ready_input();
                        fd.set_non_blocking(mode).unwrap();
                        handler.callback(io, fd, Ok(reader.ok(len)));
                        return;
                    }
                    if len == 0 {
                        fd.set_non_blocking(mode).unwrap();
                        handler.callback(io, fd, Err(eof()));
                        return;
                    }
                    let ec = errno();
                    if ec == EAGAIN {
                        fd.set_non_blocking(mode).unwrap();
                        async_read_detail(fd, buf, reader, handler);
                        return;
                    }
                    if ec != EINTR {
                        fd.as_io_actor().ready_input();
                        fd.set_non_blocking(mode).unwrap();
                        handler.callback(io, fd, Err(io::Error::from_raw_os_error(ec)));
                        return;
                    }
                }
                fd.as_io_actor().ready_input();
                handler.callback(io, fd, Err(stopped()));
            },
            ReactState::Canceled
                => handler.callback(io, fd, Err(canceled())),
            ReactState::Errored
                => unreachable!(),
        }
    }));
}

struct Read;

impl Reader for Read {
    type Output = usize;

    unsafe fn read<T: AsRawFd>(&mut self, fd: &T, buf: &mut [u8]) -> ssize_t {
        libc::read(fd.as_raw_fd(), buf.as_mut_ptr() as *mut c_void, buf.len())
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn read<T: AsIoActor>(fd: &T, buf: &mut [u8]) -> io::Result<usize> {
    read_detail(fd, buf, Read)
}

pub fn async_read<T: AsIoActor, F: Handler<T, usize>>(fd: &T, buf: &mut [u8], handler: F) {
    async_read_detail(fd, buf, Read, handler)
}

struct Recv { flags: i32 }

impl Reader for Recv {
    type Output = usize;

    unsafe fn read<T: AsRawFd>(&mut self, fd: &T, buf: &mut [u8]) -> ssize_t {
        libc::recv(fd.as_raw_fd(), buf.as_mut_ptr() as *mut c_void, buf.len(), self.flags)
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn recv<T: AsIoActor>(fd: &T, buf: &mut [u8], flags: i32) -> io::Result<usize> {
    read_detail(fd, buf, Recv { flags: flags })
}

pub fn async_recv<T: AsIoActor, F: Handler<T, usize>>(fd: &T, buf: &mut [u8], flags: i32, handler: F) {
    async_read_detail(fd, buf, Recv { flags: flags }, handler)
}

struct RecvFrom<E: Endpoint> { flags: i32, ep: E, socklen: socklen_t }

impl<E: Endpoint + Send> Reader for RecvFrom<E> {
    type Output = (usize, E);

    unsafe fn read<T: AsRawFd>(&mut self, fd: &T, buf: &mut [u8]) -> libc::ssize_t {
        libc::recvfrom(fd.as_raw_fd(), buf.as_mut_ptr() as *mut c_void, buf.len(), self.flags, self.ep.as_mut_sockaddr() as *mut _ as *mut sockaddr, &mut self.socklen)
    }

    fn ok(mut self, len: ssize_t) -> Self::Output {
        unsafe { self.ep.resize(self.socklen as usize); }
        (len as usize, self.ep)
    }
}

pub fn recvfrom<T: AsIoActor, E: Endpoint>(fd: &T, buf: &mut [u8], flags: i32, ep: E) -> io::Result<(usize, E)> {
    let socklen = ep.capacity() as socklen_t;
    read_detail(fd, buf, RecvFrom { flags: flags, ep: ep, socklen: socklen })
}

pub fn async_recvfrom<T: AsIoActor, E: Endpoint, F: Handler<T, (usize, E)>>(fd: &T, buf: &mut [u8], flags: i32,   ep: E, handler: F) {
    let socklen = ep.capacity() as socklen_t;
    async_read_detail(fd, buf, RecvFrom { flags: flags, ep: ep, socklen: socklen }, handler)
}

trait Writer : Send + 'static{
    type Output;

    unsafe fn write<T: AsRawFd>(&self, fd: &T, buf: &[u8]) -> ssize_t;

    fn ok(self, len: ssize_t) -> Self::Output;
}

fn write_detail<T: AsIoActor, W: Writer>(fd: &T, buf: &[u8], writer: W) -> io::Result<W::Output> {
    if let Some(handler) = fd.as_io_actor().unset_output() {
        handler(fd.io_service(), ReactState::Canceled);
    }

    while !fd.io_service().stopped() {
        let len = unsafe { writer.write(fd, buf) };
        if len > 0 {
            return Ok(writer.ok(len));
        }
        if len == 0 {
            return Err(write_zero());
        }
        if errno() != EINTR {
            return Err(io::Error::last_os_error());
        }
    }

    Err(stopped())
}

fn async_write_detail<T: AsIoActor, W: Writer, F: Handler<T, W::Output>>(fd: &T, buf: &[u8], writer: W, handler: F) {
    let fd_ptr = UnsafeRefCell::new(fd);
    let buf_ptr = UnsafeSliceCell::new(buf);

    fd.as_io_actor().set_output(Box::new(move |io: *const IoService, st: ReactState| {
        let io = unsafe { &*io };
        let fd = unsafe { fd_ptr.as_ref() };

        match st {
            ReactState::Ready => {
                let buf = unsafe { buf_ptr.as_slice() };
                if let Some(new_handler) = fd.as_io_actor().unset_output() {
                    io.post(|io| new_handler(io, ReactState::Ready));
                    handler.callback(io, fd, Err(canceled()));
                    return;
                }

                let mode = fd.get_non_blocking().unwrap();
                fd.set_non_blocking(true).unwrap();

                while !io.stopped() {
                    let len = unsafe { writer.write(fd, buf) };
                    if len > 0 {
                        fd.as_io_actor().ready_output();
                        fd.set_non_blocking(mode).unwrap();
                        handler.callback(io, fd, Ok(writer.ok(len)));
                        return;
                    }
                    if len == 0 {
                        fd.set_non_blocking(mode).unwrap();
                        handler.callback(io, fd, Err(eof()));
                        return;
                    }
                    let ec = errno();
                    if ec == EAGAIN {
                        fd.set_non_blocking(mode).unwrap();
                        async_write_detail(fd, buf, writer, handler);
                        return;
                    }
                    if ec != EINTR {
                        fd.as_io_actor().ready_output();
                        fd.set_non_blocking(mode).unwrap();
                        handler.callback(io, fd, Err(io::Error::from_raw_os_error(ec)));
                        return;
                    }
                }
                fd.as_io_actor().ready_output();
                handler.callback(io, fd, Err(stopped()));
            },
            ReactState::Canceled
                => handler.callback(io, fd, Err(canceled())),
            ReactState::Errored
                => unreachable!(),
        }
    }));
}

struct Write;

impl Writer for Write {
    type Output = usize;

    unsafe fn write<T: AsRawFd>(&self, fd: &T, buf: &[u8]) -> ssize_t {
        libc::write(fd.as_raw_fd(), buf.as_ptr() as *const c_void, buf.len())
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn write<T: AsIoActor>(fd: &T, buf: &[u8]) -> io::Result<usize> {
    write_detail(fd, buf, Write)
}

pub fn async_write<T: AsIoActor, F: Handler<T, usize>>(fd: &T, buf: &[u8], handler: F) {
    async_write_detail(fd, buf, Write, handler)
}

struct Sent { flags: i32 }

impl Writer for Sent {
    type Output = usize;

    unsafe fn write<T: AsRawFd>(&self, fd: &T, buf: &[u8]) -> ssize_t {
        libc::send(fd.as_raw_fd(), buf.as_ptr() as *const c_void, buf.len(), self.flags)
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn send<T: AsIoActor>(fd: &T, buf: &[u8], flags: i32) -> io::Result<usize> {
    write_detail(fd, buf, Sent { flags: flags })
}

pub fn async_send<T: AsIoActor, F: Handler<T, usize>>(fd: &T, buf: &[u8], flags: i32, handler: F) {
    async_write_detail(fd, buf, Sent { flags: flags }, handler)
}

struct SendTo<E: Endpoint> { flags: i32, ep: E }

impl<E: Endpoint + Send> Writer for SendTo<E> {
    type Output = usize;

    unsafe fn write<T: AsRawFd>(&self, fd: &T, buf: &[u8]) -> ssize_t {
        libc::sendto(fd.as_raw_fd(), buf.as_ptr() as *const c_void, buf.len(), self.flags, self.ep.as_sockaddr() as *const _ as *const sockaddr, self.ep.size() as socklen_t)
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn sendto<T: AsIoActor, E: Endpoint>(fd: &T, buf: &[u8], flags: i32, ep: E) -> io::Result<usize> {
    write_detail(fd, buf, SendTo { flags: flags, ep: ep })
}

pub fn async_sendto<T: AsIoActor, E: Endpoint, F: Handler<T, usize>>(fd: &T, buf: &[u8], flags: i32, ep: E, handler: F) {
    async_write_detail(fd, buf, SendTo { flags: flags, ep: ep }, handler)
}

pub fn cancel_io<T: AsIoActor>(fd: &T) {
    let io = fd.io_service();

    if let Some(handler) = fd.as_io_actor().unset_input() {
        io.post(|io| handler(io, ReactState::Canceled));
    }

    if let Some(handler) = fd.as_io_actor().unset_output() {
        io.post(|io| handler(io, ReactState::Canceled));
    }
}

pub fn async_wait<T: AsWaitActor, F: Handler<T, ()>>(t: &T, expiry: Expiry, handler: F) {
    let t_ptr = UnsafeRefCell::new(t);

    t.as_wait_actor().set_wait(expiry, Box::new(move |io: *const IoService, st: ReactState| {
        let io = unsafe { &*io };
        let t = unsafe { t_ptr.as_ref() };
        match st {
            ReactState::Ready
                => handler.callback(io, t, Ok(())),
            ReactState::Canceled
                => handler.callback(io, t, Err(canceled())),
            ReactState::Errored
                => unreachable!(),
        }
    }))
}

pub fn cancel_wait<T: AsWaitActor>(t: &T) {
    let io = t.io_service();
    if let Some(handler) = t.as_wait_actor().unset_wait() {
        io.post(|io| handler(io, ReactState::Canceled));
    }
}
