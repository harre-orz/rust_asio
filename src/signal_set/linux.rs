use ffi::{SystemError, INVALID_ARGUMENT, Signal, OPERATION_CANCELED, RawFd, AsRawFd, close, IN_PROGRESS, WOULD_BLOCK};
use core::{AsIoContext, IoContext, Perform, ThreadIoContext, Handle, Exec, SocketImpl};
use ops::{Complete, Handler, NoYield, Yield, AsyncReadOp};

use std::io;
use std::mem;
use std::ptr;
use std::cell::{UnsafeCell};
use libc::{sigset_t, signalfd, sigemptyset, SFD_CLOEXEC, pthread_sigmask, sigaddset, sigdelset, SIG_BLOCK, sigismember, sigprocmask, sigfillset, SIG_SETMASK, SFD_NONBLOCK};

impl Signal {
    pub fn all() -> &'static [Signal] {
        use self::Signal::*;
        &[
            SIGHUP,
            SIGINT,
            SIGQUIT,
            SIGILL,
            SIGABRT,
            SIGFPE,
            SIGSEGV,
            SIGPIPE,
            SIGALRM,
            SIGTERM,
            SIGUSR1,
            SIGUSR2,
            SIGCHLD,
            SIGCONT,
            SIGSTOP,
            SIGTSTP,
            SIGTTIN,
            SIGTTOU,
            SIGBUS,
            SIGPOLL,
            SIGPROF,
            SIGSYS,
            SIGTRAP,
            SIGURG,
            SIGVTALRM,
            SIGXCPU,
            SIGXFSZ,
        ]
    }
}

struct SignalWait<S, F> {
    sig: *const S,
    handler: F,
}

impl<S, F> SignalWait<S, F> {
    pub fn new(sig: &S, handler: F) -> Self {
        SignalWait {
            sig: sig,
            handler: handler,
        }
    }
}

unsafe impl<S, F> Send for SignalWait<S, F> {}

impl<S, F> Exec for SignalWait<S, F>
where
    S: AsRawFd + AsyncReadOp,
    F: Complete<Signal, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let sig = unsafe { &*self.sig };
        sig.add_read_op(this, Box::new(self), SystemError::default())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let sig = unsafe { &*self.sig };
        sig.add_read_op(this, self, SystemError::default())
    }
}

impl<S, F> Complete<Signal, io::Error> for SignalWait<S, F>
where
    S: AsRawFd + AsyncReadOp,
    F: Complete<Signal, io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: Signal) {
        let sig = unsafe { &*self.sig };
        sig.next_read_op(this);
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        let sig = unsafe { &*self.sig };
        sig.next_read_op(this);
        self.handler.failure(this, err)
    }
}

impl<S, F> Handler<Signal, io::Error> for SignalWait<S, F>
where
    S: AsRawFd + AsyncReadOp,
    F: Complete<Signal, io::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<S, F> Perform for SignalWait<S, F>
where
    S: AsRawFd + AsyncReadOp,
    F: Complete<Signal, io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        use libc;
        use std::mem;

        if err == SystemError::default() {
            let sig = unsafe { &*self.sig };
            while !this.as_ctx().stopped() {
                unsafe {
                    let mut ssi: libc::signalfd_siginfo = mem::uninitialized();
                    match libc::read(sig.as_raw_fd(), &mut ssi as *mut _ as *mut libc::c_void, mem::size_of_val(&ssi)) {
                        -1 => match SystemError::last_error() {
                            IN_PROGRESS | WOULD_BLOCK => return sig.add_read_op(this, self, WOULD_BLOCK),
                            INTERRUPTED => {},
                            err => return self.failure(this, err.into())
                        },
                        _ => return self.success(this, mem::transmute(ssi.ssi_signo)),
                    }
                }
            }
            self.failure(this, OPERATION_CANCELED.into())
        } else {
            self.failure(this, err.into())
        }
    }
}

pub fn async_wait<S, F>(ctx: &S, handler: F) -> F::Output
where
    S: AsRawFd + AsyncReadOp,
    F: Handler<Signal, io::Error>,
{
    let (tx, rx) = handler.channel();
    ctx.as_ctx().do_dispatch(SignalWait::new(ctx, tx));
    rx.yield_return()
}

pub type SignalImpl = SocketImpl<UnsafeCell<sigset_t>>;

impl SignalImpl {
    pub fn signal(ctx: &IoContext) -> Result<Box<Self>, SystemError> {
        let mut data = unsafe { mem::uninitialized() };
        match unsafe {
            sigemptyset(&mut data);
            signalfd(-1, &data, SFD_CLOEXEC | SFD_NONBLOCK)
        } {
            -1 => Err(SystemError::last_error()),
            fd => Ok(SignalImpl::new(ctx, fd, UnsafeCell::new(data))),
        }
    }

    pub fn add(&self, sig: Signal) -> Result<(), SystemError> {
        match unsafe {
            if sigismember(self.data.get(), sig as i32) != 0 {
                 return Err(INVALID_ARGUMENT)
            }
            sigaddset(self.data.get(), sig as i32);
            pthread_sigmask(SIG_BLOCK, self.data.get(), ptr::null_mut());
            signalfd(self.as_raw_fd(), self.data.get(), 0)
        } {
            -1 => Err(SystemError::last_error()),
            _ => Ok(())
        }
    }

    pub fn remove(&self, sig: Signal) -> Result<(), SystemError> {
        match unsafe {
            if sigismember(self.data.get(), sig as i32) == 0 {
                 return Err(INVALID_ARGUMENT)
            }
            sigdelset(self.data.get(), sig as i32);
            signalfd(self.as_raw_fd(), self.data.get(), 0)
        } {
            -1 => Err(SystemError::last_error()),
            _ => Ok(())
        }
    }

    pub fn clear(&self) {
        unsafe {
            sigemptyset(self.data.get());
            sigfillset(self.data.get());
            sigprocmask(SIG_SETMASK, self.data.get(), ptr::null_mut());
            signalfd(self.as_raw_fd(), self.data.get(), 0);
        }
    }
}

impl AsRawFd for super::SignalSet {
    fn as_raw_fd(&self) -> RawFd {
        self.pimpl.as_raw_fd()
    }
}
