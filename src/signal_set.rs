use unsafe_cell::UnsafeRefCell;
use ffi::{RawFd, AsRawFd, getnonblock, setnonblock};
use error::{ErrCode, READY, ECANCELED, EINTR, EAGAIN, last_error, eof};
use core::{IoContext, AsIoContext, ThreadIoContext, AsyncFd, workplace};
use async::{Receiver, Handler, WrappedHandler, Operation};
use reactive_io::{AsAsyncFd, AsyncInput, cancel};

use std::io;
use std::mem;
use std::ptr;
use libc::{self, SFD_CLOEXEC, SIG_SETMASK, c_void, ssize_t, sigset_t, signalfd_siginfo,
           signalfd, sigemptyset, sigaddset, sigdelset, pthread_sigmask};

/// A list specifying POSIX categories of signal.
#[repr(i32)]
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    SIGHUP = libc::SIGHUP,

    /// Interrupt from keyboard.
    SIGINT = libc::SIGINT,

    /// Quit from keyboard.
    SIGQUIT = libc::SIGQUIT,

    /// Illegal Instruction.
    SIGILL = libc::SIGILL,

    /// Abort signal from abort(3)
    SIGABRT = libc::SIGABRT,

    /// Floating point exception.
    SIGFPE = libc::SIGFPE,

    /// Kill signal.
    SIGKILL = libc::SIGKILL,

    /// Invalid memory reference.
    SIGSEGV = libc::SIGSEGV,

    /// Broken pipe: write to pipe with no readers.
    SIGPIPE = libc::SIGPIPE,

    /// Timer signal from alarm(2).
    SIGALRM = libc::SIGALRM,

    /// Termination signal.
    SIGTERM = libc::SIGTERM,

    /// User-defined signal 1.
    SIGUSR1 = libc::SIGUSR1,

    /// User-defined signal 2.
    SIGUSR2 = libc::SIGUSR2,

    /// Child stopped of terminated.
    SIGCHLD = libc::SIGCHLD,

    /// Continue if stopped.
    SIGCONT = libc::SIGCONT,

    /// Stop process.
    SIGSTOP = libc::SIGSTOP,

    /// Stop typed at terminal.
    SIGTSTP = libc::SIGTSTP,

    /// Terminal input for background process.
    SIGTTIN = libc::SIGTTIN,

    /// Terminal output for background process.
    SIGTTOU = libc::SIGTTOU,

    /// Bus error (bad memory access).
    SIGBUS = libc::SIGBUS,

    /// Pollable event (Sys V). Synonym for SIGIO.
    SIGPOLL = libc::SIGPOLL,

    /// Profiling timer expired.
    SIGPROF = libc::SIGPROF,

    /// Bad argument to routine (SVr4).
    SIGSYS = libc::SIGSYS,

    /// Trace/breakpoint trap.
    SIGTRAP = libc::SIGTRAP,

    /// Urgent condition on socket (4.2BSD).
    SIGURG = libc::SIGURG,

    /// Virtual alarm clock (4.2BSD).
    SIGVTALRM = libc::SIGVTALRM,

    /// CPU time limit exceeded (4.2BSD).
    SIGXCPU = libc::SIGXCPU,

    /// File size limit exceeded (4.2BSD).
    SIGXFSZ = libc::SIGXFSZ,
}

pub fn raise(signal: Signal) -> io::Result<()> {
    libc_try!(libc::raise(signal as i32));
    Ok(())
}

fn signalfd_init() -> io::Result<(RawFd, sigset_t)> {
    let mut mask: sigset_t = unsafe { mem::uninitialized() };
    libc_ign!(sigemptyset(&mut mask));
    let sfd = libc_try!(signalfd(-1, &mask, SFD_CLOEXEC));
    Ok((sfd, mask))
}

fn signalfd_add(sfd: RawFd, mask: &mut sigset_t, signal: Signal) -> io::Result<()> {
    libc_try!(sigaddset(mask, signal as i32));
    libc_ign!(pthread_sigmask(SIG_SETMASK, mask, ptr::null_mut()));
    libc_ign!(signalfd(sfd, mask, 0));
    Ok(())
}

fn signalfd_del(sfd: RawFd, mask: &mut sigset_t, signal: Signal) -> io::Result<()> {
    libc_try!(sigdelset(mask, signal as i32));
    libc_ign!(pthread_sigmask(SIG_SETMASK, mask, ptr::null_mut()));
    libc_ign!(signalfd(sfd, mask, 0));
    Ok(())
}

fn signalfd_reset(sfd: RawFd, mask: &mut sigset_t) -> io::Result<()> {
    libc_try!(sigemptyset(mask));
    libc_ign!(pthread_sigmask(SIG_SETMASK, mask, ptr::null_mut()));
    libc_ign!(signalfd(sfd, mask, 0));
    Ok(())
}

unsafe fn signalfd_read(sfd: RawFd, ssi: &mut signalfd_siginfo) -> ssize_t
{
    libc::read(sfd, ssi as *mut _ as *mut c_void, mem::size_of_val(ssi))
}

fn signalfd_wait<T>(sfd: &T) -> io::Result<Signal>
    where T: AsyncInput,
{
    while !sfd.as_ctx().stopped() {
        let mut ssi: signalfd_siginfo = unsafe { mem::uninitialized() };
        let len = unsafe { signalfd_read(sfd.as_raw_fd(), &mut ssi) };
        if len > 0 {
            return Ok(unsafe { mem::transmute(ssi.ssi_signo) });
        }
        if len == 0 {
            return Err(eof());
        }
        let ec = last_error();
        if ec != EINTR {
            return Err(ec.into());
        }
    }
    Err(ECANCELED.into())
}

struct SignalHandler<T> {
    sfd: UnsafeRefCell<T>
}

impl<T> WrappedHandler<Signal, io::Error> for SignalHandler<T>
    where T: AsyncInput,
{
    fn perform(&mut self, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode, op: Operation<Signal, io::Error, Self>) {
        let sfd = unsafe { self.sfd.as_ref() };
        match ec {
            READY => {
                let mode = getnonblock(sfd).unwrap();
                setnonblock(sfd, true).unwrap();

                while !ctx.stopped() {
                    let mut ssi: signalfd_siginfo = unsafe { mem::uninitialized() };
                    let len = unsafe { signalfd_read(sfd.as_raw_fd(), &mut ssi) };
                    if len > 0 {
                        setnonblock(sfd, mode).unwrap();
                        sfd.next_op(this);
                        op.send(ctx, Ok(unsafe { mem::transmute(ssi.ssi_signo) }));
                        return;
                    }
                    if len == 0 {
                        setnonblock(sfd, mode).unwrap();
                        sfd.next_op(this);
                        op.send(ctx, Err(eof()));
                        return;
                    }

                    let ec = last_error();
                    if ec == EAGAIN {
                        setnonblock(sfd, mode).unwrap();
                        sfd.add_op(this, op, ec);
                        return;
                    }
                    if ec != EINTR {
                        setnonblock(sfd, mode).unwrap();
                        sfd.next_op(this);
                        op.send(ctx, Err(ec.into()));
                        return;
                    }
                }

                setnonblock(sfd, mode).unwrap();
                sfd.next_op(this);
                op.send(ctx, Err(ECANCELED.into()));
                return;
            },
            ec => op.send(ctx, Err(ec.into())),
        }
    }
}

fn signalfd_async_wait<T, F>(sfd: &T, handler: F) -> F::Output
    where T: AsyncInput,
          F: Handler<Signal, io::Error>,
{
    let (op, res) = handler.channel(SignalHandler { sfd: UnsafeRefCell::new(sfd) });
    workplace(sfd.as_ctx(), |this| sfd.add_op(this, op, READY));
    res.recv(sfd.as_ctx())
}

/// Provides a signal handing.
pub struct SignalSet {
    fd: AsyncFd,
    mask: sigset_t,
}

impl Drop for SignalSet {
    fn drop(&mut self) {
        signalfd_reset(self.fd.as_raw_fd(), &mut self.mask).unwrap();
    }
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> io::Result<SignalSet> {
        let (fd, mask) = try!(signalfd_init());
        Ok(SignalSet {
            fd: AsyncFd::new::<Self>(fd, ctx),
            mask: mask,
        })
    }

    pub fn add(&mut self, signal: Signal) -> io::Result<()> {
        signalfd_add(self.as_raw_fd(), &mut self.mask, signal)
    }

    pub fn async_wait<F>(&self, handler: F) -> F::Output
        where F: Handler<Signal, io::Error>,
    {
        signalfd_async_wait(self, handler)
    }

    pub fn cancel(&self) -> &Self {
        cancel(self);
        self
    }

    pub fn clear(&mut self) -> io::Result<()> {
        signalfd_reset(self.as_raw_fd(), &mut self.mask)
    }

    pub fn remove(&mut self, signal: Signal) -> io::Result<()> {
        signalfd_del(self.as_raw_fd(), &mut self.mask, signal)
    }

    pub fn wait(&self) -> io::Result<Signal> {
        signalfd_wait(self)
    }
}

impl AsRawFd for SignalSet {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

unsafe impl Send for SignalSet { }

unsafe impl AsIoContext for SignalSet {
    fn as_ctx(&self) -> &IoContext {
        self.fd.as_ctx()
    }
}

impl AsAsyncFd for SignalSet {
    fn as_fd(&self) -> &AsyncFd {
        &self.fd
    }
}

#[test]
fn test_signal_set() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    let mut sig = SignalSet::new(ctx).unwrap();
    sig.add(Signal::SIGHUP).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR2).unwrap();
}

#[test]
fn test_signal_set_wait() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    let mut sig = SignalSet::new(ctx).unwrap();
    sig.add(Signal::SIGHUP).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    raise(Signal::SIGHUP).unwrap();
    raise(Signal::SIGUSR1).unwrap();
    assert_eq!(sig.wait().unwrap(), Signal::SIGHUP);
    assert_eq!(sig.wait().unwrap(), Signal::SIGUSR1);
}
