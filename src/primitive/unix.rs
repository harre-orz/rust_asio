use crate::error::{OsError, Result};
use crate::primitive::Timeout;
use std::ffi::CStr;
use std::num::NonZero;
use std::time::Instant;

pub(crate) struct Fd(libc::c_int);

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

impl Fd {
    pub(crate) const unsafe fn from_raw_fd(fd: libc::c_int) -> Self {
        Self(fd)
    }

    pub(crate) const unsafe fn as_raw_fd(&self) -> libc::c_int {
        self.0
    }

    pub(crate) fn close(self) -> Result<()> {
        let Fd(fd) = self;
        unsafe {
            match libc::close(fd) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub(crate) fn open(filename: &CStr) -> Result<Self> {
        unsafe {
            let flags = 0;
            #[cfg(target_os = "linux")]
            let flags = flags | libc::O_CLOEXEC | libc::O_NONBLOCK;
            match libc::open(filename.as_ptr(), flags) {
                -1 => Err(OsError::last()),
                fd => {
                    #[cfg(target_os = "macos")]
                    fd.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    fd.set_nonblock()?;
                    Ok(Self(fd))
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_cloexec(&self) -> Result<()> {
        unsafe {
            match libc::fcntl(self.0, libc::F_SETFD, libc::FD_CLOEXEC) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_nonblock(&self) -> Result<()> {
        unsafe {
            match libc::fcntl(self.0, libc::F_SETFL, libc::O_NONBLOCK) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub(crate) fn read(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match libc::read(self.0, buf.as_mut_ptr().cast(), buf.len()) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub(crate) fn write(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match libc::write(self.0, buf.as_ptr().cast(), buf.len()) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }
}

/// Low-level UNIX-based socket type.
pub struct Socket(pub(crate) Fd);

#[cfg(doc)]
impl Drop for Socket {
    fn drop(&mut self) {}
}

/// A list specifying POSIX categories of signal.
#[cfg(unix)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Signal(NonZero<libc::c_int>);

#[cfg(unix)]
impl Signal {
    const fn new(signo: libc::c_int) -> Self {
        Self(NonZero::new(signo).unwrap())
    }

    /// Hangup detected on controlling terminal or death of controlling process.
    pub const HUP: Self = Self::new(libc::SIGHUP);

    /// Interrupt from keyboard.
    pub const INT: Self = Self::new(libc::SIGINT);

    /// Quit from keyboard.
    pub const QUIT: Self = Self::new(libc::SIGQUIT);

    /// Illegal Instruction.
    pub const ILL: Self = Self::new(libc::SIGILL);

    /// Abort signal from abort(3)
    pub const ABRT: Self = Self::new(libc::SIGABRT);

    /// Floating point exception.
    pub const FPE: Self = Self::new(libc::SIGFPE);

    /// Kill signal.
    pub const KILL: Self = Self::new(libc::SIGKILL);

    /// Invalid memory reference.
    pub const SEGV: Self = Self::new(libc::SIGSEGV);

    /// Broken pipe: write to pipe with no readers.
    pub const PIPE: Self = Self::new(libc::SIGPIPE);

    /// Timer signal from alarm(2).
    pub const ALRM: Self = Self::new(libc::SIGALRM);

    /// Termination signal.
    pub const TERM: Self = Self::new(libc::SIGTERM);

    /// User-defined signal 1.
    pub const USR1: Self = Self::new(libc::SIGUSR1);

    /// User-defined signal 2.
    pub const USR2: Self = Self::new(libc::SIGUSR2);

    /// Child stopped of terminated.
    pub const CHLD: Self = Self::new(libc::SIGCHLD);

    /// Continue if stopped.
    pub const CONT: Self = Self::new(libc::SIGCONT);

    /// Stop process.
    pub const STOP: Self = Self::new(libc::SIGSTOP);

    /// Stop typed at terminal.
    pub const TSTP: Self = Self::new(libc::SIGTSTP);

    /// Terminal input for background process.
    pub const TTIN: Self = Self::new(libc::SIGTTIN);

    /// Terminal output for background process.
    pub const TTOU: Self = Self::new(libc::SIGTTOU);

    /// Bus error (bad memory access).
    pub const BUS: Self = Self::new(libc::SIGBUS);

    /// Pollable event (Sys V). Synonym for SIGIO.
    #[cfg(target_os = "linux")]
    pub const POLL: Self = Self::new(libc::SIGPOLL);

    /// Profiling timer expired.
    pub const PROF: Self = Self::new(libc::SIGPROF);

    /// Bad argument to routine (SVr4).
    pub const SYS: Self = Self::new(libc::SIGSYS);

    /// Trace/breakpoint trap.
    pub const TRAP: Self = Self::new(libc::SIGTRAP);

    /// Urgent condition on socket (4.2BSD).
    pub const URG: Self = Self::new(libc::SIGURG);

    /// Virtual alarm clock (4.2BSD).
    pub const VTALRM: Self = Self::new(libc::SIGVTALRM);

    /// CPU time limit exceeded (4.2BSD).
    pub const XCPU: Self = Self::new(libc::SIGXCPU);

    /// File size limit exceeded (4.2BSD).
    pub const XFSZ: Self = Self::new(libc::SIGXFSZ);

    pub const fn number(&self) -> i32 {
        self.0.get()
    }

    #[cfg(target_os = "linux")]
    pub(crate) const unsafe fn from_signalfd_siginfo(ssi: &libc::signalfd_siginfo) -> Self {
        Self::new(ssi.ssi_signo as libc::c_int)
    }

    #[cfg(target_os = "macos")]
    pub(crate) const unsafe fn from_kevent(kev: &libc::kevent) -> Self {
        Self::new(kev.ident as libc::c_int)
    }
}
