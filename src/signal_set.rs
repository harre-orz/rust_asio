use crate::{ffi, ops, IoContext, OsError};
use std::os::fd::OwnedFd;
use std::time::Duration;

/// A list specifying POSIX categories of signal.
#[repr(i32)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    HUP = libc::SIGHUP,

    /// Interrupt from keyboard.
    INT = libc::SIGINT,

    /// Quit from keyboard.
    QUIT = libc::SIGQUIT,

    /// Illegal Instruction.
    ILL = libc::SIGILL,

    /// Abort signal from abort(3)
    ABRT = libc::SIGABRT,

    /// Floating point exception.
    FPE = libc::SIGFPE,

    /// Kill signal.
    KILL = libc::SIGKILL,

    /// Invalid memory reference.
    SEGV = libc::SIGSEGV,

    /// Broken pipe: write to pipe with no readers.
    PIPE = libc::SIGPIPE,

    /// Timer signal from alarm(2).
    ALRM = libc::SIGALRM,

    /// Termination signal.
    TERM = libc::SIGTERM,

    /// User-defined signal 1.
    USR1 = libc::SIGUSR1,

    /// User-defined signal 2.
    USR2 = libc::SIGUSR2,

    /// Child stopped of terminated.
    CHLD = libc::SIGCHLD,

    /// Continue if stopped.
    CONT = libc::SIGCONT,

    /// Stop process.
    STOP = libc::SIGSTOP,

    /// Stop typed at terminal.
    TSTP = libc::SIGTSTP,

    /// Terminal input for background process.
    TTIN = libc::SIGTTIN,

    /// Terminal output for background process.
    TTOU = libc::SIGTTOU,

    /// Bus error (bad memory access).
    BUS = libc::SIGBUS,

    /// Pollable event (Sys V). Synonym for SIGIO.
    #[cfg(target_os = "linux")]
    POLL = libc::SIGPOLL,

    /// Profiling timer expired.
    PROF = libc::SIGPROF,

    /// Bad argument to routine (SVr4).
    SYS = libc::SIGSYS,

    /// Trace/breakpoint trap.
    TRAP = libc::SIGTRAP,

    /// Urgent condition on socket (4.2BSD).
    URG = libc::SIGURG,

    /// Virtual alarm clock (4.2BSD).
    VTALRM = libc::SIGVTALRM,

    /// CPU time limit exceeded (4.2BSD).
    XCPU = libc::SIGXCPU,

    /// File size limit exceeded (4.2BSD).
    XFSZ = libc::SIGXFSZ,
}

impl Signal {
    pub(crate) const unsafe fn from_raw(signo: u32) -> Self {
        match signo as i32 {
            libc::SIGHUP => Self::HUP,
            libc::SIGINT => Self::INT,
            libc::SIGQUIT => Self::QUIT,
            libc::SIGILL => Self::ILL,
            libc::SIGABRT => Self::ABRT,
            libc::SIGFPE => Self::FPE,
            libc::SIGKILL => Self::KILL,
            libc::SIGSEGV => Self::SEGV,
            libc::SIGPIPE => Self::PIPE,
            libc::SIGALRM => Self::ALRM,
            libc::SIGTERM => Self::TERM,
            libc::SIGUSR1 => Self::USR1,
            libc::SIGUSR2 => Self::USR2,
            libc::SIGCHLD => Self::CHLD,
            libc::SIGCONT => Self::CONT,
            libc::SIGSTOP => Self::STOP,
            libc::SIGTSTP => Self::TSTP,
            libc::SIGTTIN => Self::TTIN,
            libc::SIGTTOU => Self::TTOU,
            libc::SIGBUS => Self::BUS,
            #[cfg(target_os = "linux")]
            libc::SIGPOLL => Self::POLL,
            libc::SIGPROF => Self::PROF,
            libc::SIGSYS => Self::SYS,
            libc::SIGTRAP => Self::TRAP,
            libc::SIGURG => Self::URG,
            libc::SIGVTALRM => Self::VTALRM,
            libc::SIGXCPU => Self::XCPU,
            libc::SIGXFSZ => Self::XFSZ,
            _ => panic!(),
        }
    }
}


impl Into<i32> for Signal {
    fn into(self) -> i32 {
        self as i32
    }
}

pub struct SignalSetBuilder {
    ctx: IoContext,
    set: libc::sigset_t,
    err: Option<OsError>,
}

impl SignalSetBuilder {
    pub fn add(mut self, signal: Signal) -> Self {
        if self.err.is_none() {
            if let Err(err) = ffi::sigaddset(&mut self.set, signal) {
                self.err = Some(err);
            }
        }
        self
    }

    pub fn any(mut self) -> Self {
        if self.err.is_none() {
            self.set = ffi::sigfillset();
        }
        self
    }

    pub fn ready(self) -> Result<SignalSet, OsError> {
        if let Some(err) = self.err {
            return Err(err);
        }

        let _ = ffi::sigprocmask(libc::SIG_BLOCK, &self.set)?;
        let sfd = ffi::signalfd(&self.set)?;
        Ok(SignalSet::new_priv(self.ctx, sfd))
    }
}

pub struct SignalSet {
    ctx: IoContext,
    sfd: OwnedFd,
    wait_timeout: Duration,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> SignalSetBuilder {
        SignalSetBuilder {
            ctx: ctx.clone(),
            set: ffi::sigemptyset(),
            err: None,
        }
    }

    fn new_priv(ctx: IoContext, sfd: OwnedFd) -> Self {
        Self {
            ctx: ctx,
            sfd: sfd,
            wait_timeout: Duration::MAX,
        }
    }

    pub fn close(self) -> Result<(), OsError> {
        ffi::close(self.sfd)
    }

    pub fn nb_signal_read(&mut self) -> Result<Signal, OsError> {
        ffi::signal_read(&self.sfd)
    }

    pub fn signal_read(&self) -> Result<Signal, OsError> {
        ops::signal_read(&self.ctx, &self.sfd, self.wait_timeout)
    }

    pub async fn async_signal_read(&self) -> Result<Signal, OsError> {
        ops::async_signal_read(&self.ctx, &self.sfd, self.wait_timeout).await
    }
}
