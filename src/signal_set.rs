use crate::error::{OsError, Result};
use crate::exec::AsyncSocket;
use crate::socket::{Fd, Socket, Timeout};
use crate::{IoContext, ops};
use std::mem::MaybeUninit;
use std::num::NonZero;
use std::slice;
use std::time::Duration;

/// A list specifying POSIX categories of signal.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Signal {
    signo: NonZero<i32>,
}

impl Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    pub const HUP: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGHUP) },
    };

    /// Interrupt from keyboard.
    pub const INT: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGINT) },
    };

    /// Quit from keyboard.
    pub const QUIT: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGQUIT) },
    };

    /// Illegal Instruction.
    pub const ILL: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGILL) },
    };

    /// Abort signal from abort(3)
    pub const ABRT: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGABRT) },
    };

    /// Floating point exception.
    pub const FPE: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGFPE) },
    };

    /// Kill signal.
    pub const KILL: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGKILL) },
    };

    /// Invalid memory reference.
    pub const SEGV: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGSEGV) },
    };

    /// Broken pipe: write to pipe with no readers.
    pub const PIPE: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGPIPE) },
    };

    /// Timer signal from alarm(2).
    pub const ALRM: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGALRM) },
    };

    /// Termination signal.
    pub const TERM: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGTERM) },
    };

    /// User-defined signal 1.
    pub const USR1: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGUSR1) },
    };

    /// User-defined signal 2.
    pub const USR2: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGUSR2) },
    };

    /// Child stopped of terminated.
    pub const CHLD: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGCHLD) },
    };

    /// Continue if stopped.
    pub const CONT: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGCONT) },
    };

    /// Stop process.
    pub const STOP: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGSTOP) },
    };

    /// Stop typed at terminal.
    pub const TSTP: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGTSTP) },
    };

    /// Terminal input for background process.
    pub const TTIN: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGTTIN) },
    };

    /// Terminal output for background process.
    pub const TTOU: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGTTOU) },
    };

    /// Bus error (bad memory access).
    pub const BUS: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGBUS) },
    };

    /// Pollable event (Sys V). Synonym for SIGIO.
    #[cfg(target_os = "linux")]
    pub const POLL: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGPOLL) },
    };

    /// Profiling timer expired.
    pub const PROF: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGPROF) },
    };

    /// Bad argument to routine (SVr4).
    pub const SYS: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGSYS) },
    };

    /// Trace/breakpoint trap.
    pub const TRAP: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGTRAP) },
    };

    /// Urgent condition on socket (4.2BSD).
    pub const URG: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGURG) },
    };

    /// Virtual alarm clock (4.2BSD).
    pub const VTALRM: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGVTALRM) },
    };

    /// CPU time limit exceeded (4.2BSD).
    pub const XCPU: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGXCPU) },
    };

    /// File size limit exceeded (4.2BSD).
    pub const XFSZ: Self = Self {
        signo: unsafe { NonZero::new_unchecked(libc::SIGXFSZ) },
    };
}

fn sigemptyset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigemptyset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

#[allow(dead_code)]
fn sigfillset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigfillset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

fn sigaddset(mask: &mut libc::sigset_t, sig: Signal) {
    unsafe {
        libc::sigaddset(mask, sig.signo.get());
    }
}

fn sigdelset(mask: &mut libc::sigset_t, sig: Signal) {
    unsafe {
        libc::sigdelset(mask, sig.signo.get());
    }
}

#[allow(dead_code)]
fn sigprocmask(how: i32, set: &libc::sigset_t) -> Result<libc::sigset_t> {
    let mut oset = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        match libc::sigprocmask(how, set, oset.as_mut_ptr()) {
            -1 => Err(OsError::last()),
            _ => Ok(oset.assume_init()),
        }
    }
}

fn signalfd(mask: &libc::sigset_t) -> Result<Socket> {
    unsafe {
        match libc::signalfd(-1, mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) {
            -1 => Err(OsError::last()),
            sfd => Ok(Socket::from_raw_fd(Fd::new_unchecked(sfd))),
        }
    }
}

fn nb_wait(soc: &Socket) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        soc.read(buf)?;
        let ssi = ssi.assume_init();
        Ok(Signal {
            signo: NonZero::new_unchecked(ssi.ssi_signo as i32),
        })
    }
}

async fn async_wait(soc: &AsyncSocket, timeout: Timeout) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        ops::async_read_some(&soc, buf, timeout).await?;
        let ssi = ssi.assume_init();
        Ok(Signal {
            signo: NonZero::new_unchecked(ssi.ssi_signo as i32),
        })
    }
}

fn wait(ctx: &IoContext, soc: &Socket, timeout: Timeout) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        ops::read_some(ctx, soc, buf, timeout)?;
        let ssi = ssi.assume_init();
        Ok(Signal {
            signo: NonZero::new_unchecked(ssi.ssi_signo as i32),
        })
    }
}

pub struct AsyncSignalSet {
    sfd: AsyncSocket,
    timeout: Timeout,
}

impl AsyncSignalSet {
    pub fn as_ctx(&self) -> &IoContext {
        self.sfd.as_ctx()
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }
    pub fn nb_wait(&self) -> Result<Signal> {
        nb_wait(self.sfd.as_socket())
    }

    pub async fn async_wait(&self) -> Result<Signal> {
        async_wait(&self.sfd, self.timeout).await
    }
}

pub struct SignalSet {
    sfd: Socket,
    ctx: IoContext,
    timeout: Timeout,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> SignalSetBuilder {
        SignalSetBuilder {
            ctx: ctx.clone(),
            mask: sigemptyset(),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }
    pub fn nb_wait(&self) -> Result<Signal> {
        nb_wait(&self.sfd)
    }

    pub fn wait(&self) -> Result<Signal> {
        wait(&self.ctx, &self.sfd, self.timeout)
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(sfd: SignalSet) -> Self {
        Self {
            sfd: AsyncSocket::new(sfd.ctx, sfd.sfd),
            timeout: sfd.timeout,
        }
    }
}

pub struct SignalSetBuilder {
    ctx: IoContext,
    mask: libc::sigset_t,
}

impl SignalSetBuilder {
    pub fn add(mut self, sig: Signal) -> Self {
        sigaddset(&mut self.mask, sig);
        self
    }

    pub fn del(mut self, sig: Signal) -> Self {
        sigdelset(&mut self.mask, sig);
        self
    }

    pub fn listen(self) -> Result<SignalSet> {
        let sfd = signalfd(&self.mask)?;
        Ok(SignalSet {
            sfd: sfd,
            ctx: self.ctx,
            timeout: Timeout::infinite(),
        })
    }
}
