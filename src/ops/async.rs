use std::io;
use std::slice;
use {UnsafeThreadableCell, IoObject, IoService, Strand, AsSockAddr, NonBlocking};
use ops;
use ops::AsyncResult;
use backbone::{Handler, HandlerResult, Expiry, IoActor, TimerActor};

fn operation_canceled() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Operation canceled")
}

fn connection_refused() -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused")
}

fn post_cancel<T: IoObject>(io: &T, id: usize, callback: Handler) {
    io.io_service().0.task.post(id, Box::new(|io: *const IoService| {
        callback(unsafe { &*io }, HandlerResult::Canceled)
    }))
}

pub trait AsIoActor : IoObject + NonBlocking {
    fn as_io_actor(&self) -> &IoActor;
}

pub fn cancel_io<S: AsIoActor>(soc: &S) {
    let mut id = 0;
    if let Some(callback) = soc.as_io_actor().unset_in(&mut id) {
        post_cancel(soc, id, callback)
    }
    if let Some(callback) = soc.as_io_actor().unset_out(&mut id) {
        post_cancel(soc, id, callback)
    }
}

pub fn syncd_connect<S: AsIoActor, E: AsSockAddr>(soc: &S, ep: &E) -> io::Result<()> {
    let mut _id = 0;
    if let Some(callback) = soc.as_io_actor().unset_out(&mut _id) {
        callback(soc.io_service(), HandlerResult::Canceled)
    }
    try!(soc.set_native_non_blocking(soc.get_non_blocking()));
    ops::connect(soc, ep)
}

pub fn async_connect<S, E, F, T>(soc: &S, ep: &E, callback: F, obj: &Strand<T>)
    where S: AsIoActor + NonBlocking + 'static,
          E: AsSockAddr + 'static,
          F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
          T: 'static,
{
    let io = obj.io_service();
    assert_eq!(io, soc.io_service());

    let mut _id = 0;
    if let Some(callback) = soc.as_io_actor().unset_out(&mut _id) {
        io.0.task.post(obj.id(), Box::new(move |io| callback(io, HandlerResult::Canceled)));
    }
    match ops::connect_with_nonblock(soc, ep) {
        AsyncResult::Err(err) => {
            io.post_strand(move |obj| callback(obj, Err(err)), obj);
        },
        AsyncResult::Ok(_) => {
            io.post_strand(move |obj| callback(obj, Ok(())), obj);
        },
        AsyncResult::WouldBlock => {
            let obj_ = obj.obj.clone();
            soc.as_io_actor().set_out(obj.id(), Box::new(move |io: *const IoService, res| {
                let obj = Strand { io: unsafe { &*io }, obj: obj_ };
                match res {
                    HandlerResult::Ready =>
                        callback(obj, Ok(())),
                    HandlerResult::Canceled =>
                        callback(obj, Err(operation_canceled())),
                    HandlerResult::Errored =>
                        callback(obj, Err(connection_refused())),
                }
            }));
        },
    }
}

pub fn syncd_accept<S, E>(soc: &S, ep: E) -> io::Result<(ops::RawFd, E)>
    where S: AsIoActor + NonBlocking,
          E: AsSockAddr,
{
    let mut _id = 0;
    if let Some(callback) = soc.as_io_actor().unset_in(&mut _id) {
        callback(soc.io_service(), HandlerResult::Canceled);
    }
    try!(soc.set_native_non_blocking(soc.get_non_blocking()));
    ops::accept(soc, ep)
}

pub fn async_accept<S, E, F, T>(soc: &S, mut ep: E, callback: F, obj: &Strand<T>)
    where S: AsIoActor + NonBlocking + 'static,
          E: AsSockAddr + Clone + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<(ops::RawFd, E)>) + Send + 'static,
          T: 'static,
{
    let io = obj.io_service();
    assert_eq!(io, soc.io_service());

    if soc.as_io_actor().ready_in(false) {
        match ops::accept_with_nonblock(soc, &mut ep) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_in(true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_in(true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new(soc as *const S);
    soc.as_io_actor().set_in(obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        let soc = unsafe { &**ptr_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled
                => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready
                => callback(obj, ops::accept(soc, ep)),
        }
    }));
}

pub fn syncd_read<S: AsIoActor>(soc: &S, buf: &mut [u8]) -> io::Result<usize> {
    let mut _id = 0;
    if let Some(callback) = soc.as_io_actor().unset_in(&mut _id) {
        callback(soc.io_service(), HandlerResult::Canceled)
    }
    try!(soc.set_native_non_blocking(soc.get_non_blocking()));
    ops::read(soc, buf)
}

pub fn async_read<S, F, T>(soc: &S, buf: &mut [u8], callback: F, obj: &Strand<T>)
    where S: AsIoActor + NonBlocking + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static,
{
    let io = obj.io_service();
    assert_eq!(io, soc.io_service());

    if soc.as_io_actor().ready_in(false) {
        match ops::read_with_nonblock(soc, buf) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_in(true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_in(true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_mut_ptr(), buf.len()));
    soc.as_io_actor().set_in(obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts_mut(ptr_.1, ptr_.2) };
                    ops::read(soc, buf)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn syncd_recv<S: AsIoActor>(soc: &S, buf: &mut [u8], flags: i32) -> io::Result<usize> {
    let mut _id = 0;
    if let Some(callback) = soc.as_io_actor().unset_in(&mut _id) {
        callback(soc.io_service(), HandlerResult::Canceled)
    }
    try!(soc.set_native_non_blocking(soc.get_non_blocking()));
    ops::recv(soc, buf, flags)
}


pub fn async_recv<S, F, O>(soc: &S, buf: &mut [u8], flags: i32, callback: F, obj: &Strand<O>)
    where S: AsIoActor + NonBlocking + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    assert_eq!(io, soc.io_service());

    if soc.as_io_actor().ready_in(false) {
        match ops::recv_with_nonblock(soc, buf, flags) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_in(true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_in(true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_mut_ptr(), buf.len()));
    soc.as_io_actor().set_in(obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts_mut(ptr_.1, ptr_.2) };
                    ops::recv(soc, buf, flags)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn syncd_recvfrom<S: AsIoActor, E: AsSockAddr>(soc: &S, buf: &mut [u8], flags: i32, ep: E) -> io::Result<(usize, E)> {
    let mut _id = 0;
    if let Some(callback) = soc.as_io_actor().unset_in(&mut _id) {
        callback(soc.io_service(), HandlerResult::Canceled)
    }
    try!(soc.set_native_non_blocking(soc.get_non_blocking()));
    ops::recvfrom(soc, buf, flags, ep)
}

pub fn async_recvfrom<S, E, F, O>(soc: &S, buf: &mut [u8], flags: i32, mut ep: E, callback: F, obj: &Strand<O>)
    where S: AsIoActor + NonBlocking + 'static,
          E: AsSockAddr + Clone + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<(usize, E)>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    assert_eq!(io, soc.io_service());

    if soc.as_io_actor().ready_in(false) {
        match ops::recvfrom_with_nonblock(soc, buf, flags, &mut ep) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_in(true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_in(true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_mut_ptr(), buf.len()));
    soc.as_io_actor().set_in(obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts_mut(ptr_.1, ptr_.2) };
                    ops::recvfrom(soc, buf, flags, ep)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn syncd_write<S: AsIoActor>(soc: &S, buf: &[u8]) -> io::Result<usize> {
    let mut _id = 0;
    if let Some(callback) = soc.as_io_actor().unset_in(&mut _id) {
        callback(soc.io_service(), HandlerResult::Canceled)
    }
    try!(soc.set_native_non_blocking(soc.get_non_blocking()));
    ops::write(soc, buf)
}

pub fn async_write<S, F, O>(soc: &S, buf: &[u8], callback: F, obj: &Strand<O>)
    where S: AsIoActor + NonBlocking + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    assert_eq!(io, soc.io_service());

    if soc.as_io_actor().ready_out(false) {
        match ops::write_with_nonblock(soc, buf) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_out(true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_out(true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_ptr(), buf.len()));
    soc.as_io_actor().set_out(obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts(ptr_.1, ptr_.2) };
                    ops::write(soc, buf)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn syncd_send<S: AsIoActor>(soc: &S, buf: &[u8], flags: i32) -> io::Result<usize> {
    let mut _id = 0;
    if let Some(callback) = soc.as_io_actor().unset_in(&mut _id) {
        callback(soc.io_service(), HandlerResult::Canceled)
    }
    try!(soc.set_native_non_blocking(soc.get_non_blocking()));
    ops::send(soc, buf, flags)
}

pub fn async_send<S, F, O>(soc: &S, buf: &[u8], flags: i32, callback: F, obj: &Strand<O>)
    where S: AsIoActor + NonBlocking + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    assert_eq!(io, soc.io_service());

    if soc.as_io_actor().ready_out(false) {
        match ops::send_with_nonblock(soc, buf, flags) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_out(true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_out(true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_ptr(), buf.len()));
    soc.as_io_actor().set_out(obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts(ptr_.1, ptr_.2) };
                    ops::send(soc, buf, flags)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn syncd_sendto<S: AsIoActor, E: AsSockAddr>(soc: &S, buf: &[u8], flags: i32, ep: &E) -> io::Result<usize> {
    let mut _id = 0;
    if let Some(callback) = soc.as_io_actor().unset_in(&mut _id) {
        callback(soc.io_service(), HandlerResult::Canceled)
    }
    try!(soc.set_native_non_blocking(soc.get_non_blocking()));
    ops::sendto(soc, buf, flags, ep)
}

pub fn async_sendto<S, E, F, O>(soc: &S, buf: &[u8], flags: i32, ep: &E, callback: F, obj: &Strand<O>)
    where S: AsIoActor + NonBlocking + 'static,
          E: AsSockAddr + Clone + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    assert_eq!(io, soc.io_service());

    if soc.as_io_actor().ready_out(false) {
        match ops::sendto_with_nonblock(soc, buf, flags, ep) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_out(true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            },
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_out(true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            },
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_ptr(), buf.len()));
    let ep_ = ep.clone();
    soc.as_io_actor().set_out(obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled
                => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts(ptr_.1, ptr_.2) };
                    ops::sendto(soc, buf, flags, &ep_)
                };
                callback(obj, res);
            },
        }
    }));
}

pub trait AsTimerActor : IoObject {
    fn as_timer_actor(&self) -> &TimerActor;
}

pub fn cancel_wait<W: AsTimerActor>(wait: &W) {
    if let Some((id, callback)) = wait.as_timer_actor().unset_timer() {
        post_cancel(wait, id, callback)
    }
}

pub fn async_wait<W, F, T>(wait: &W, expiry: Expiry, callback: F, strand: &Strand<T>)
    where W: AsTimerActor + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<()>) + Send + 'static,
          T: 'static {
    let obj_ = strand.obj.clone();
    wait.as_timer_actor().set_timer(expiry, strand.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Ready
                => callback(obj, Ok(())),
            HandlerResult::Canceled
                => callback(obj, Err(operation_canceled())),
            HandlerResult::Errored
                => unreachable!(),
        }
    }));
}
