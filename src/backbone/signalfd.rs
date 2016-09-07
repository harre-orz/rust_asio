use std::io;
use std::mem;
use std::ptr;
use libc;
use libc::{EINTR, EAGAIN, c_void, signalfd_siginfo};
use {UnsafeRefCell, IoService, Handler};
use super::{ErrorCode, READY, CANCELED, RawFd, AsRawFd, AsIoActor, errno, getnonblock, setnonblock,
            eof, stopped, canceled};

pub use libc::{sigset_t};

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    SIGHUP = libc::SIGHUP as isize,

    /// Interrupt from keyboard.
    SIGINT = libc::SIGINT as isize,

    /// Quit from keyboard.
    SIGQUIT = libc::SIGQUIT as isize,

    /// Illegal Instruction.
    SIGILL = libc::SIGILL as isize,

    /// Abort signal from abort(3)
    SIGABRT = libc::SIGABRT as isize,

    /// Floating point exception.
    SIGFPE = libc::SIGFPE as isize,

    /// Kill signal.
    SIGKILL = libc::SIGKILL as isize,

    /// Invalid memory reference.
    SIGSEGV = libc::SIGSEGV as isize,

    /// Broken pipe: write to pipe with no readers.
    SIGPIPE = libc::SIGPIPE as isize,

    /// Timer signal from alarm(2).
    SIGALRM = libc::SIGALRM as isize,

    /// Termination signal.
    SIGTERM = libc::SIGTERM as isize,

    /// User-defined signal 1.
    SIGUSR1 = libc::SIGUSR1 as isize,

    /// User-defined signal 2.
    SIGUSR2 = libc::SIGUSR2 as isize,

    /// Child stopped of terminated.
    SIGCHLD = libc::SIGCHLD as isize,

    /// Continue if stopped.
    SIGCONT = libc::SIGCONT as isize,

    /// Stop process.
    SIGSTOP = libc::SIGSTOP as isize,

    /// Stop typed at terminal.
    SIGTSTP = libc::SIGTSTP as isize,

    /// Terminal input for background process.
    SIGTTIN = libc::SIGTTIN as isize,

    /// Terminal output for background process.
    SIGTTOU = libc::SIGTTOU as isize,

    /// Bus error (bad memory access).
    SIGBUS = libc::SIGBUS as isize,

    /// Pollable event (Sys V). Synonym for SIGIO.
    SIGPOLL = libc::SIGPOLL as isize,

    /// Profiling timer expired.
    SIGPROF = libc::SIGPROF as isize,

    /// Bad argument to routine (SVr4).
    SIGSYS = libc::SIGSYS as isize,

    /// Trace/breakpoint trap.
    SIGTRAP = libc::SIGTRAP as isize,

    /// Urgent condition on socket (4.2BSD).
    SIGURG = libc::SIGURG as isize,

    /// Virtual alarm clock (4.2BSD).
    SIGVTALRM = libc::SIGVTALRM as isize,

    /// CPU time limit exceeded (4.2BSD).
    SIGXCPU = libc::SIGXCPU as isize,

    /// File size limit exceeded (4.2BSD).
    SIGXFSZ = libc::SIGXFSZ as isize,
}

pub fn raise(signal: Signal) -> io::Result<()> {
    libc_try!(libc::raise(signal as i32));
    Ok(())
}

pub fn signalfd_init() -> io::Result<(RawFd, sigset_t)> {
    let mut mask: sigset_t = unsafe { mem::uninitialized() };
    unsafe { libc::sigemptyset(&mut mask) };
    let fd = libc_try!(libc::signalfd(-1, &mask, libc::SFD_CLOEXEC));
    Ok((fd, mask))
}

pub fn signalfd_add<T: AsRawFd>(fd: &T, mask: &mut sigset_t, signal: Signal) -> io::Result<()> {
    libc_try!(libc::sigaddset(mask, signal as i32));
    unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, mask, ptr::null_mut()) };
    unsafe { libc::signalfd(fd.as_raw_fd(), mask, 0) };
    Ok(())
}

pub fn signalfd_del<T: AsRawFd>(fd: &T, mask: &mut sigset_t, signal: Signal) -> io::Result<()> {
    libc_try!(libc::sigdelset(mask, signal as i32));
    unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, mask, ptr::null_mut()) };
    unsafe { libc::signalfd(fd.as_raw_fd(), mask, 0) };
    Ok(())
}

pub fn signalfd_reset<T: AsRawFd>(fd: &T, mask: &mut sigset_t) -> io::Result<()> {
    libc_try!(libc::sigemptyset(mask));
    unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, mask, ptr::null_mut()) };
    unsafe { libc::signalfd(fd.as_raw_fd(), mask, 0) };
    Ok(())
}

pub fn signalfd_read<T: AsIoActor>(fd: &T) -> io::Result<Signal> {
    if let Some(handler) = fd.as_io_actor().unset_input(false) {
        handler(fd.io_service(), ErrorCode(CANCELED));
    }

    while !fd.io_service().stopped() {
        let mut ssi: libc::signalfd_siginfo = unsafe { mem::uninitialized() };
        let len = unsafe { libc::read(
            fd.as_raw_fd(),
            &mut ssi as *mut _ as *mut c_void,
            mem::size_of::<signalfd_siginfo>()
        ) };
        if len > 0 {
            return Ok(unsafe { mem::transmute(ssi.ssi_signo as i8) });
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

pub fn signalfd_async_read<T: AsIoActor, F: Handler<Signal>>(fd: &T, handler: F) {
    let fd_ptr = UnsafeRefCell::new(fd);

    fd.as_io_actor().set_input(Box::new(move |io: *const IoService, ec: ErrorCode| {
        let io = unsafe { &*io };

        match ec.0 {
            READY => {
                let fd = unsafe { fd_ptr.as_ref() };
                let mut ssi: signalfd_siginfo = unsafe { mem::uninitialized() };

                if let Some(new_handler) = fd.as_io_actor().unset_input(false) {
                    io.post(|io| new_handler(io, ErrorCode(READY)));
                    handler.callback(io, Err(canceled()));
                    return;
                }

                let mode = getnonblock(fd).unwrap();
                setnonblock(fd, true).unwrap();

                while !io.stopped() {
                    let len = unsafe { libc::read(
                        fd.as_raw_fd(),
                        &mut ssi as *mut _ as *mut c_void,
                        mem::size_of::<signalfd_siginfo>()
                    ) };
                    if len > 0 {
                        fd.as_io_actor().ready_input();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Ok(unsafe { mem::transmute(ssi.ssi_signo as u8) }));
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
                        signalfd_async_read(fd, handler);
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
