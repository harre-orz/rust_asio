use crate::error::{OsError, Result};
use std::ffi::CStr;
use std::num::NonZero;

/// A list specifying POSIX categories of signal.
#[cfg(unix)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Signal(NonZero<libc::c_int>);

#[cfg(unix)]
impl Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    pub const HUP: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGHUP) });

    /// Interrupt from keyboard.
    pub const INT: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGINT) });

    /// Quit from keyboard.
    pub const QUIT: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGQUIT) });

    /// Illegal Instruction.
    pub const ILL: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGILL) });

    /// Abort signal from abort(3)
    pub const ABRT: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGABRT) });

    /// Floating point exception.
    pub const FPE: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGFPE) });

    /// Kill signal.
    pub const KILL: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGKILL) });

    /// Invalid memory reference.
    pub const SEGV: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGSEGV) });

    /// Broken pipe: write to pipe with no readers.
    pub const PIPE: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGPIPE) });

    /// Timer signal from alarm(2).
    pub const ALRM: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGALRM) });

    /// Termination signal.
    pub const TERM: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTERM) });

    /// User-defined signal 1.
    pub const USR1: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGUSR1) });

    /// User-defined signal 2.
    pub const USR2: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGUSR2) });

    /// Child stopped of terminated.
    pub const CHLD: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGCHLD) });

    /// Continue if stopped.
    pub const CONT: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGCONT) });

    /// Stop process.
    pub const STOP: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGSTOP) });

    /// Stop typed at terminal.
    pub const TSTP: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTSTP) });

    /// Terminal input for background process.
    pub const TTIN: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTTIN) });

    /// Terminal output for background process.
    pub const TTOU: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTTOU) });

    /// Bus error (bad memory access).
    pub const BUS: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGBUS) });

    /// Pollable event (Sys V). Synonym for SIGIO.
    #[cfg(target_os = "linux")]
    pub const POLL: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGPOLL) });

    /// Profiling timer expired.
    pub const PROF: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGPROF) });

    /// Bad argument to routine (SVr4).
    pub const SYS: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGSYS) });

    /// Trace/breakpoint trap.
    pub const TRAP: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGTRAP) });

    /// Urgent condition on socket (4.2BSD).
    pub const URG: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGURG) });

    /// Virtual alarm clock (4.2BSD).
    pub const VTALRM: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGVTALRM) });

    /// CPU time limit exceeded (4.2BSD).
    pub const XCPU: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGXCPU) });

    /// File size limit exceeded (4.2BSD).
    pub const XFSZ: Self = Self(unsafe { NonZero::new_unchecked(libc::SIGXFSZ) });

    pub const fn number(&self) -> i32 {
        self.0.get()
    }

    #[cfg(target_os = "linux")]
    pub(crate) const unsafe fn from_signalfd_siginfo(ssi: &libc::signalfd_siginfo) -> Self {
        Self(unsafe { NonZero::new_unchecked(ssi.ssi_signo as i32) })
    }

    #[cfg(target_os = "macos")]
    pub(crate) unsafe fn from_kevent(kev: &libc::kevent) -> Self {
        Self(unsafe { NonZero::new_unchecked(kev.ident as libc::c_int) })
    }
}

pub struct Fd(pub(crate) libc::c_int);

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

impl Fd {
    pub unsafe fn new_unchecked(fd: libc::c_int) -> Self {
        Self(fd)
    }

    pub const unsafe fn as_raw_fd(&self) -> libc::c_int {
        self.0
    }

    pub fn open(filename: &CStr) -> Result<Self> {
        unsafe {
            #[cfg(target_os = "linux")]
            let flags = libc::O_CLOEXEC | libc::O_NONBLOCK;
            #[cfg(target_os = "macos")]
            let flags = 0;
            match libc::open(filename.as_ptr(), flags) {
                -1 => Err(OsError::last()),
                soc => {
                    let fd = Self(soc);
                    #[cfg(target_os = "macos")]
                    fd.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    fd.set_nonblock()?;
                    Ok(fd)
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
    fn set_nonblock(&self) -> Result<()> {
        unsafe {
            match libc::fcntl(self.0, libc::F_SETFL, libc::O_NONBLOCK) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match libc::read(self.0, buf.as_mut_ptr().cast(), buf.len()) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match libc::write(self.0, buf.as_ptr().cast(), buf.len()) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }
}
