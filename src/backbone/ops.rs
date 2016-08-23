use std::io;
use libc;
use libc::{EINTR, EAGAIN, EINPROGRESS, c_void, sockaddr, socklen_t, ssize_t};
use {UnsafeRefCell, UnsafeSliceCell, IoService, SockAddr, Handler};
use super::{ErrorCode, READY, CANCELED,
            RawFd, AsRawFd, AsIoActor, AsWaitActor, Expiry, errno, getnonblock, setnonblock,
            eof, write_zero, stopped, canceled};

pub fn connect<T: AsIoActor, E: SockAddr>(fd: &T, ep: &E) -> io::Result<()> {
    if let Some(handler) = fd.as_io_actor().unset_output() {
        handler(fd.io_service(), ErrorCode(CANCELED));
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

pub fn async_connect<T: AsIoActor, E: SockAddr, F: Handler<()>>(fd: &T, ep: &E, handler: F) {
    let io = fd.io_service();
    if let Some(handler) = fd.as_io_actor().unset_output() {
        io.post(move |io| handler(io, ErrorCode(CANCELED)));
    }

    let mode = getnonblock(fd).unwrap();
    setnonblock(fd, true).unwrap();
    while !fd.io_service().stopped() {
        if unsafe { libc::connect(
            fd.as_raw_fd(),
            ep.as_sockaddr() as *const _ as *const sockaddr,
            ep.size() as socklen_t
        ) } == 0 {
            setnonblock(fd, mode).unwrap();
            io.post(move |io| handler.callback(io, Ok(())));
            return;
        }

        let ec = errno();
        if ec == EINPROGRESS {
            let fd_ptr = UnsafeRefCell::new(fd);
            fd.as_io_actor().set_output(Box::new(move |io: *const IoService, ec: ErrorCode| {
                let io = unsafe { &*io };
                let fd = unsafe { fd_ptr.as_ref() };
                fd.as_io_actor().ready_output();
                setnonblock(fd, mode).unwrap();
                handler.callback(io, match ec.0 {
                    READY => Ok(()),
                    CANCELED => Err(canceled()),
                    ec => Err(io::Error::from_raw_os_error(ec)),
                });
            }));
            return;
        }
        if ec != EINTR {
            setnonblock(fd, mode).unwrap();
            io.post(move |io| handler.callback(io, Err(io::Error::from_raw_os_error(ec))));
            return;
        }
    }

    setnonblock(fd, mode).unwrap();
    io.post(move |io| handler.callback(io, Err(stopped())));
}

pub fn accept<T: AsIoActor, E: SockAddr>(fd: &T, mut ep: E) -> io::Result<(RawFd, E)> {
    if let Some(handler) = fd.as_io_actor().unset_input() {
        handler(fd.io_service(), ErrorCode(CANCELED));
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

pub fn async_accept<T: AsIoActor, E: SockAddr, F: Handler<(RawFd, E)>>(fd: &T, mut ep: E, handler: F) {
    let fd_ptr = UnsafeRefCell::new(fd);

    fd.as_io_actor().set_input(Box::new(move |io: *const IoService, ec: ErrorCode| {
        let io = unsafe { &*io };
        match ec.0 {
            READY => {
                let fd = unsafe { fd_ptr.as_ref() };
                if let Some(new_handler) = fd.as_io_actor().unset_input() {
                    handler.callback(io, Err(canceled()));
                    new_handler(io, ErrorCode(READY));
                    return;
                }

                let mode = getnonblock(fd).unwrap();
                setnonblock(fd, true).unwrap();

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
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Ok((acc, ep)));
                        return;
                    }
                    let ec = errno();
                    if ec == EAGAIN {
                        setnonblock(fd, mode).unwrap();
                        async_accept(fd, ep, handler);
                        return;
                    }
                    if ec != EINTR {
                        fd.as_io_actor().ready_input();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Err(io::Error::from_raw_os_error(ec)));
                        return;
                    }
                }
                fd.as_io_actor().ready_input();
                setnonblock(fd, mode).unwrap();
                handler.callback(io, Err(stopped()));
            },
            CANCELED => handler.callback(io, Err(canceled())),
            ec => handler.callback(io, Err(io::Error::from_raw_os_error(ec))),
        }
    }));
}

trait Reader : Send + 'static {
    type Output;

    unsafe fn read<T: AsRawFd>(&mut self, fd: &T, buf: &mut [u8]) -> ssize_t;

    fn ok(self, len: ssize_t) -> Self::Output;
}

fn read_detail<T:AsIoActor, R: Reader>(fd: &T, buf: &mut [u8], mut reader: R) -> io::Result<R::Output> {
    if let Some(handler) = fd.as_io_actor().unset_input() {
        handler(fd.io_service(), ErrorCode(CANCELED));
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

fn async_read_detail<T: AsIoActor, R: Reader, F: Handler<R::Output>>(fd: &T, buf: &mut [u8], mut reader: R, handler: F) {
    let fd_ptr = UnsafeRefCell::new(fd);
    let buf_ptr = UnsafeSliceCell::new(buf);

    fd.as_io_actor().set_input(Box::new(move |io: *const IoService, ec: ErrorCode| {
        let io = unsafe { &*io };

        match ec.0 {
            READY => {
                let fd = unsafe { fd_ptr.as_ref() };
                let buf = unsafe { buf_ptr.as_slice_mut() };

                if let Some(new_handler) = fd.as_io_actor().unset_input() {
                    io.post(|io| new_handler(io, ErrorCode(READY)));
                    handler.callback(io, Err(canceled()));
                    return;
                }

                let mode = getnonblock(fd).unwrap();
                setnonblock(fd, true).unwrap();

                while !io.stopped() {
                    let len = unsafe { reader.read(fd, buf) };
                    if len > 0 {
                        fd.as_io_actor().ready_input();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Ok(reader.ok(len)));
                        return;
                    }
                    if len == 0 {
                        fd.as_io_actor().ready_input();
                        handler.callback(io, Err(eof()));
                        return;
                    }
                    let ec = errno();
                    if ec == EAGAIN {
                        setnonblock(fd, mode).unwrap();
                        async_read_detail(fd, buf, reader, handler);
                        return;
                    }
                    if ec != EINTR {
                        fd.as_io_actor().ready_input();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Err(io::Error::from_raw_os_error(ec)));
                        return;
                    }
                }
                fd.as_io_actor().ready_input();
                setnonblock(fd, mode).unwrap();
                handler.callback(io, Err(stopped()));
            },
            CANCELED => handler.callback(io, Err(canceled())),
            ec => handler.callback(io, Err(io::Error::from_raw_os_error(ec))),
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

pub fn async_read<T: AsIoActor, F: Handler<usize>>(fd: &T, buf: &mut [u8], handler: F) {
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

pub fn async_recv<T: AsIoActor, F: Handler<usize>>(fd: &T, buf: &mut [u8], flags: i32, handler: F) {
    async_read_detail(fd, buf, Recv { flags: flags }, handler)
}

struct RecvFrom<E: SockAddr> { flags: i32, ep: E, socklen: socklen_t }

impl<E: SockAddr + Send> Reader for RecvFrom<E> {
    type Output = (usize, E);

    unsafe fn read<T: AsRawFd>(&mut self, fd: &T, buf: &mut [u8]) -> libc::ssize_t {
        libc::recvfrom(fd.as_raw_fd(), buf.as_mut_ptr() as *mut c_void, buf.len(), self.flags, self.ep.as_mut_sockaddr() as *mut _ as *mut sockaddr, &mut self.socklen)
    }

    fn ok(mut self, len: ssize_t) -> Self::Output {
        unsafe { self.ep.resize(self.socklen as usize); }
        (len as usize, self.ep)
    }
}

pub fn recvfrom<T: AsIoActor, E: SockAddr>(fd: &T, buf: &mut [u8], flags: i32, ep: E) -> io::Result<(usize, E)> {
    let socklen = ep.capacity() as socklen_t;
    read_detail(fd, buf, RecvFrom { flags: flags, ep: ep, socklen: socklen })
}

pub fn async_recvfrom<T: AsIoActor, E: SockAddr, F: Handler<(usize, E)>>(fd: &T, buf: &mut [u8], flags: i32,   ep: E, handler: F) {
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
        handler(fd.io_service(), ErrorCode(CANCELED));
    }

    while !fd.io_service().stopped() {
        let len = unsafe { writer.write(fd, buf) };
        if len > 0 {
            return Ok(writer.ok(len));
        }
        if len == 0 {
            return Err(write_zero());
        }
        let ec = errno();
        if ec != EINTR {
            return Err(io::Error::from_raw_os_error(ec));
        }
    }

    Err(stopped())
}

fn async_write_detail<T: AsIoActor, W: Writer, F: Handler<W::Output>>(fd: &T, buf: &[u8], writer: W, handler: F) {
    let fd_ptr = UnsafeRefCell::new(fd);
    let buf_ptr = UnsafeSliceCell::new(buf);

    fd.as_io_actor().set_output(Box::new(move |io: *const IoService, ec: ErrorCode| {
        let io = unsafe { &*io };

        match ec.0 {
            READY => {
                let fd = unsafe { fd_ptr.as_ref() };
                let buf = unsafe { buf_ptr.as_slice() };
                if let Some(new_handler) = fd.as_io_actor().unset_output() {
                    io.post(|io| new_handler(io, ErrorCode(READY)));
                    handler.callback(io, Err(canceled()));
                    return;
                }

                let mode = getnonblock(fd).unwrap();
                setnonblock(fd, true).unwrap();

                while !io.stopped() {
                    let len = unsafe { writer.write(fd, buf) };
                    if len > 0 {
                        fd.as_io_actor().ready_output();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Ok(writer.ok(len)));
                        return;
                    }
                    if len == 0 {
                        fd.as_io_actor().ready_output();
                        handler.callback(io, Err(eof()));
                        return;
                    }
                    let ec = errno();
                    if ec == EAGAIN {
                        setnonblock(fd, mode).unwrap();
                        async_write_detail(fd, buf, writer, handler);
                        return;
                    }
                    if ec != EINTR {
                        fd.as_io_actor().ready_output();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Err(io::Error::from_raw_os_error(ec)));
                        return;
                    }
                }
                fd.as_io_actor().ready_output();
                setnonblock(fd, mode).unwrap();
                handler.callback(io, Err(stopped()));
            },
            CANCELED => handler.callback(io, Err(canceled())),
            ec => handler.callback(io, Err(io::Error::from_raw_os_error(ec))),

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

pub fn async_write<T: AsIoActor, F: Handler<usize>>(fd: &T, buf: &[u8], handler: F) {
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

pub fn async_send<T: AsIoActor, F: Handler<usize>>(fd: &T, buf: &[u8], flags: i32, handler: F) {
    async_write_detail(fd, buf, Sent { flags: flags }, handler)
}

struct SendTo<E: SockAddr> { flags: i32, ep: E }

impl<E: SockAddr + Send> Writer for SendTo<E> {
    type Output = usize;

    unsafe fn write<T: AsRawFd>(&self, fd: &T, buf: &[u8]) -> ssize_t {
        libc::sendto(fd.as_raw_fd(), buf.as_ptr() as *const c_void, buf.len(), self.flags, self.ep.as_sockaddr() as *const _ as *const sockaddr, self.ep.size() as socklen_t)
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn sendto<T: AsIoActor, E: SockAddr>(fd: &T, buf: &[u8], flags: i32, ep: E) -> io::Result<usize> {
    write_detail(fd, buf, SendTo { flags: flags, ep: ep })
}

pub fn async_sendto<T: AsIoActor, E: SockAddr, F: Handler<usize>>(fd: &T, buf: &[u8], flags: i32, ep: E, handler: F) {
    async_write_detail(fd, buf, SendTo { flags: flags, ep: ep }, handler)
}

pub fn cancel_io<T: AsIoActor>(fd: &T) {
    let io = fd.io_service();

    if let Some(handler) = fd.as_io_actor().unset_input() {
        io.post(|io| handler(io, ErrorCode(CANCELED)));
    }

    if let Some(handler) = fd.as_io_actor().unset_output() {
        io.post(|io| handler(io, ErrorCode(CANCELED)));
    }
}

pub fn async_wait<T: AsWaitActor, F: Handler<()>>(t: &T, expiry: Expiry, handler: F) {
    t.as_wait_actor().set_wait(expiry, Box::new(move |io: *const IoService, st: ErrorCode| {
        let io = unsafe { &*io };
        match st.0 {
            READY => handler.callback(io, Ok(())),
            CANCELED => handler.callback(io, Err(canceled())),
            ec => handler.callback(io, Err(io::Error::from_raw_os_error(ec))),
        }
    }))
}

pub fn cancel_wait<T: AsWaitActor>(t: &T) {
    let io = t.io_service();
    if let Some(handler) = t.as_wait_actor().unset_wait() {
        io.post(|io| handler(io, ErrorCode(CANCELED)));
    }
}
