use crate::{ffi, ops, IoContext};
use crate::error::OsError;
use std::os::fd::OwnedFd;
use std::time::Duration;

/// A list specifying POSIX categories of signal.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Signal {
    pub(crate) signo: u32,
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

    pub async fn async_signal_read(&self) -> Result<Signal, OsError> {
        ops::async_signal_read(&self.ctx, &self.sfd, self.wait_timeout).await
    }

    pub fn close(self) -> Result<(), OsError> {
        ffi::close(self.sfd)
    }

    pub fn nb_signal_read(&self) -> Result<Signal, OsError> {
        ffi::signal_read(&self.sfd)
    }

    pub fn signal_read(&self) -> Result<Signal, OsError> {
        ops::signal_read(&self.ctx, &self.sfd, self.wait_timeout)
    }
}
