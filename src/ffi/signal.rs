use crate::error::OsError;
use std::mem::MaybeUninit;

type Result<T> = std::result::Result<T, OsError>;

/// A list specifying POSIX categories of signal.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Signal {
    pub(super) signo: u32,
}

impl Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    pub const HUP: Self = Self::from_raw(libc::SIGHUP);

    /// Interrupt from keyboard.
    pub const INT: Self = Self::from_raw(libc::SIGINT);

    /// Quit from keyboard.
    pub const QUIT: Self = Self::from_raw(libc::SIGQUIT);

    /// Illegal Instruction.
    pub const ILL: Self = Self::from_raw(libc::SIGILL);

    /// Abort signal from abort(3)
    pub const ABRT: Self = Self::from_raw(libc::SIGABRT);

    /// Floating point exception.
    pub const FPE: Self = Self::from_raw(libc::SIGFPE);

    /// Kill signal.
    pub const KILL: Self = Self::from_raw(libc::SIGKILL);

    /// Invalid memory reference.
    pub const SEGV: Self = Self::from_raw(libc::SIGSEGV);

    /// Broken pipe: write to pipe with no readers.
    pub const PIPE: Self = Self::from_raw(libc::SIGPIPE);

    /// Timer signal from alarm(2).
    pub const ALRM: Self = Self::from_raw(libc::SIGALRM);

    /// Termination signal.
    pub const TERM: Self = Self::from_raw(libc::SIGTERM);

    /// User-defined signal 1.
    pub const USR1: Self = Self::from_raw(libc::SIGUSR1);

    /// User-defined signal 2.
    pub const USR2: Self = Self::from_raw(libc::SIGUSR2);

    /// Child stopped of terminated.
    pub const CHLD: Self = Self::from_raw(libc::SIGCHLD);

    /// Continue if stopped.
    pub const CONT: Self = Self::from_raw(libc::SIGCONT);

    /// Stop process.
    pub const STOP: Self = Self::from_raw(libc::SIGSTOP);

    /// Stop typed at terminal.
    pub const TSTP: Self = Self::from_raw(libc::SIGTSTP);

    /// Terminal input for background process.
    pub const TTIN: Self = Self::from_raw(libc::SIGTTIN);

    /// Terminal output for background process.
    pub const TTOU: Self = Self::from_raw(libc::SIGTTOU);

    /// Bus error (bad memory access).
    pub const BUS: Self = Self::from_raw(libc::SIGBUS);

    /// Pollable event (Sys V). Synonym for SIGIO.
    #[cfg(target_os = "linux")]
    pub const POLL: Self = Self::from_raw(libc::SIGPOLL);

    /// Profiling timer expired.
    pub const PROF: Self = Self::from_raw(libc::SIGPROF);

    /// Bad argument to routine (SVr4).
    pub const SYS: Self = Self::from_raw(libc::SIGSYS);

    /// Trace/breakpoint trap.
    pub const TRAP: Self = Self::from_raw(libc::SIGTRAP);

    /// Urgent condition on socket (4.2BSD).
    pub const URG: Self = Self::from_raw(libc::SIGURG);

    /// Virtual alarm clock (4.2BSD).
    pub const VTALRM: Self = Self::from_raw(libc::SIGVTALRM);

    /// CPU time limit exceeded (4.2BSD).
    pub const XCPU: Self = Self::from_raw(libc::SIGXCPU);

    /// File size limit exceeded (4.2BSD).
    pub const XFSZ: Self = Self::from_raw(libc::SIGXFSZ);

    const fn from_raw(signo: i32) -> Self {
        Self {
            signo: signo as u32,
        }
    }
}

impl Into<i32> for Signal {
    fn into(self) -> i32 {
        self.signo as i32
    }
}

pub fn sigemptyset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigemptyset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

pub fn sigfillset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigfillset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

pub fn sigaddset(mask: &mut libc::sigset_t, sig: Signal) -> Result<()> {
    match unsafe { libc::sigaddset(mask, sig.into()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

pub fn sigprocmask(how: i32, set: &libc::sigset_t) -> Result<libc::sigset_t> {
    let mut oset = MaybeUninit::<libc::sigset_t>::uninit();
    match unsafe { libc::sigprocmask(how, set, oset.as_mut_ptr()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(unsafe { oset.assume_init() }),
        _ => unreachable!(),
    }
}
