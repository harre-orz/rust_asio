use ffi::{SystemError};
use handler::Handler;
use ops::{AsyncReadOp, async_signal_wait};
use core::{AsIoContext, IoContext, InnerSignal, ThreadIoContext, Perform};

use std::io;

pub use ffi::{Signal, raise};

pub struct SignalSet {
    inner: Box<InnerSignal>,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> io::Result<Self> {
        Ok(SignalSet {
            inner: InnerSignal::new(ctx),
        })
    }

    pub fn add(&self, sig: Signal) -> io::Result<()> {
        Ok(self.inner.add(sig)?)
    }

    pub fn async_wait<F>(&self, handler: F) -> F::Output
        where F: Handler<Signal, io::Error>
    {
        async_signal_wait(self, handler)
    }

    pub fn cancel(&self) {
        self.inner.cancel()
    }

    pub fn clear(&self) {
        self.inner.clear()
    }

    pub fn remove(&self, sig: Signal) -> io::Result<()> {
        Ok(self.inner.remove(sig)?)
    }
}

unsafe impl Send for SignalSet {}

unsafe impl Sync for SignalSet {}

unsafe impl AsIoContext for SignalSet {
    fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }
}

impl AsyncReadOp for SignalSet {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.inner.add_read_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.inner.next_read_op(this)
    }
}


// use unsafe_cell::UnsafeRefCell;
// use ffi::{RawFd, AsRawFd, getnonblock, setnonblock};
// use error::{ErrCode, READY, ECANCELED, EINTR, EAGAIN, last_error, eof};
// use core::{IoContext, AsIoContext, ThreadIoContext, AsyncFd, workplace};
// use async::{Receiver, Handler, WrappedHandler, Operation};
// use reactive_io::{AsAsyncFd, AsyncInput, cancel};
//
// use std::io;
// use std::mem;
// use std::ptr;
// use libc::{self, SFD_CLOEXEC, SIG_SETMASK, c_void, ssize_t, sigset_t, signalfd_siginfo,
//            signalfd, sigemptyset, sigaddset, sigdelset, pthread_sigmask};
//
//
// pub fn raise(signal: Signal) -> io::Result<()> {
//     libc_try!(libc::raise(signal as i32));
//     Ok(())
// }
//
// fn signalfd_init() -> io::Result<(RawFd, sigset_t)> {
//     let mut mask: sigset_t = unsafe { mem::uninitialized() };
//     libc_ign!(sigemptyset(&mut mask));
//     let sfd = libc_try!(signalfd(-1, &mask, SFD_CLOEXEC));
//     Ok((sfd, mask))
// }
//
// fn signalfd_add(sfd: RawFd, mask: &mut sigset_t, signal: Signal) -> io::Result<()> {
//     libc_try!(sigaddset(mask, signal as i32));
//     libc_ign!(pthread_sigmask(SIG_SETMASK, mask, ptr::null_mut()));
//     libc_ign!(signalfd(sfd, mask, 0));
//     Ok(())
// }
//
// fn signalfd_del(sfd: RawFd, mask: &mut sigset_t, signal: Signal) -> io::Result<()> {
//     libc_try!(sigdelset(mask, signal as i32));
//     libc_ign!(pthread_sigmask(SIG_SETMASK, mask, ptr::null_mut()));
//     libc_ign!(signalfd(sfd, mask, 0));
//     Ok(())
// }
//
// fn signalfd_reset(sfd: RawFd, mask: &mut sigset_t) -> io::Result<()> {
//     libc_try!(sigemptyset(mask));
//     libc_ign!(pthread_sigmask(SIG_SETMASK, mask, ptr::null_mut()));
//     libc_ign!(signalfd(sfd, mask, 0));
//     Ok(())
// }
//
// unsafe fn signalfd_read(sfd: RawFd, ssi: &mut signalfd_siginfo) -> ssize_t
// {
//     libc::read(sfd, ssi as *mut _ as *mut c_void, mem::size_of_val(ssi))
// }
//
// fn signalfd_wait<T>(sfd: &T) -> io::Result<Signal>
//     where T: AsyncInput,
// {
//     while !sfd.as_ctx().stopped() {
//         let mut ssi: signalfd_siginfo = unsafe { mem::uninitialized() };
//         let len = unsafe { signalfd_read(sfd.as_raw_fd(), &mut ssi) };
//         if len > 0 {
//             return Ok(unsafe { mem::transmute(ssi.ssi_signo) });
//         }
//         if len == 0 {
//             return Err(eof());
//         }
//         let ec = last_error();
//         if ec != EINTR {
//             return Err(ec.into());
//         }
//     }
//     Err(ECANCELED.into())
// }
//
// struct SignalHandler<T> {
//     sfd: UnsafeRefCell<T>
// }
//
// impl<T> WrappedHandler<Signal, io::Error> for SignalHandler<T>
//     where T: AsyncInput,
// {
//     fn perform(&mut self, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode, op: Operation<Signal, io::Error, Self>) {
//         let sfd = unsafe { self.sfd.as_ref() };
//         match ec {
//             READY => {
//                 let mode = getnonblock(sfd).unwrap();
//                 setnonblock(sfd, true).unwrap();
//
//                 while !ctx.stopped() {
//                     let mut ssi: signalfd_siginfo = unsafe { mem::uninitialized() };
//                     let len = unsafe { signalfd_read(sfd.as_raw_fd(), &mut ssi) };
//                     if len > 0 {
//                         setnonblock(sfd, mode).unwrap();
//                         sfd.next_op(this);
//                         op.send(ctx, Ok(unsafe { mem::transmute(ssi.ssi_signo) }));
//                         return;
//                     }
//                     if len == 0 {
//                         setnonblock(sfd, mode).unwrap();
//                         sfd.next_op(this);
//                         op.send(ctx, Err(eof()));
//                         return;
//                     }
//
//                     let ec = last_error();
//                     if ec == EAGAIN {
//                         setnonblock(sfd, mode).unwrap();
//                         sfd.add_op(this, op, ec);
//                         return;
//                     }
//                     if ec != EINTR {
//                         setnonblock(sfd, mode).unwrap();
//                         sfd.next_op(this);
//                         op.send(ctx, Err(ec.into()));
//                         return;
//                     }
//                 }
//
//                 setnonblock(sfd, mode).unwrap();
//                 sfd.next_op(this);
//                 op.send(ctx, Err(ECANCELED.into()));
//                 return;
//             },
//             ec => op.send(ctx, Err(ec.into())),
//         }
//     }
// }
//
// fn signalfd_async_wait<T, F>(sfd: &T, handler: F) -> F::Output
//     where T: AsyncInput,
//           F: Handler<Signal, io::Error>,
// {
//     let (op, res) = handler.channel(SignalHandler { sfd: UnsafeRefCell::new(sfd) });
//     workplace(sfd.as_ctx(), |this| sfd.add_op(this, op, READY));
//     res.recv(sfd.as_ctx())
// }
//
// /// Provides a signal handing.
// pub struct SignalSet {
//     fd: AsyncFd,
//     mask: sigset_t,
// }
//
// impl Drop for SignalSet {
//     fn drop(&mut self) {
//         signalfd_reset(self.fd.as_raw_fd(), &mut self.mask).unwrap();
//     }
// }
//
// impl SignalSet {
//     pub fn new(ctx: &IoContext) -> io::Result<SignalSet> {
//         let (fd, mask) = try!(signalfd_init());
//         Ok(SignalSet {
//             fd: AsyncFd::new::<Self>(fd, ctx),
//             mask: mask,
//         })
//     }
//
//     pub fn add(&mut self, signal: Signal) -> io::Result<()> {
//         signalfd_add(self.as_raw_fd(), &mut self.mask, signal)
//     }
//
//     pub fn async_wait<F>(&self, handler: F) -> F::Output
//         where F: Handler<Signal, io::Error>,
//     {
//         signalfd_async_wait(self, handler)
//     }
//
//     pub fn cancel(&self) -> &Self {
//         cancel(self);
//         self
//     }
//
//     pub fn clear(&mut self) -> io::Result<()> {
//         signalfd_reset(self.as_raw_fd(), &mut self.mask)
//     }
//
//     pub fn remove(&mut self, signal: Signal) -> io::Result<()> {
//         signalfd_del(self.as_raw_fd(), &mut self.mask, signal)
//     }
//
//     pub fn wait(&self) -> io::Result<Signal> {
//         signalfd_wait(self)
//     }
// }
//
// impl AsRawFd for SignalSet {
//     fn as_raw_fd(&self) -> RawFd {
//         self.fd.as_raw_fd()
//     }
// }
//
// unsafe impl Send for SignalSet { }
//
// unsafe impl AsIoContext for SignalSet {
//     fn as_ctx(&self) -> &IoContext {
//         self.fd.as_ctx()
//     }
// }
//
// impl AsAsyncFd for SignalSet {
//     fn as_fd(&self) -> &AsyncFd {
//         &self.fd
//     }
// }
//
#[test]
fn test_signal_set() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    let mut sig = SignalSet::new(ctx).unwrap();
    sig.add(Signal::SIGHUP).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGHUP).unwrap();
}

#[test]
#[should_panic]
fn test_signal_set_dup_add() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    let mut sig = SignalSet::new(ctx).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
}

#[test]
#[should_panic]
fn test_signal_set_dup_del() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    let mut sig = SignalSet::new(ctx).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR1).unwrap();
}

#[test]
fn test_signal_set_wait() {
    use core::IoContext;
    use handler::wrap;
    use std::sync::Arc;
    use std::thread;

    let ctx = &IoContext::new().unwrap();
    let mut sig = Arc::new(SignalSet::new(ctx).unwrap());
    sig.add(Signal::SIGHUP).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.async_wait(wrap(|ctx, sig: io::Result<Signal>| assert_eq!(sig.unwrap(), Signal::SIGHUP), &sig));
    sig.async_wait(wrap(|ctx, sig: io::Result<Signal>| assert_eq!(sig.unwrap(), Signal::SIGUSR1), &sig));
    raise(Signal::SIGHUP).unwrap();
    raise(Signal::SIGUSR1).unwrap();
    ctx.run();
}
