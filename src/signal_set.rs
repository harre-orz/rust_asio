use crate::IoContext;
use crate::error::OsError;
use crate::exec::async_socket::AsyncSocket;
use crate::ffi::socket::Socket;
use std::cell::Cell;
use std::mem::MaybeUninit;
use std::num::NonZero;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, OsError>;

/// A list specifying POSIX categories of signal.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Signal {
    signo: NonZero<i32>,
}

impl Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    pub const HUP: Self = Self {
        signo: NonZero::new(libc::SIGHUP).unwrap(),
    };

    /// Interrupt from keyboard.
    pub const INT: Self = Self {
        signo: NonZero::new(libc::SIGINT).unwrap(),
    };

    /// Quit from keyboard.
    pub const QUIT: Self = Self {
        signo: NonZero::new(libc::SIGQUIT).unwrap(),
    };

    /// Illegal Instruction.
    pub const ILL: Self = Self {
        signo: NonZero::new(libc::SIGILL).unwrap(),
    };

    /// Abort signal from abort(3)
    pub const ABRT: Self = Self {
        signo: NonZero::new(libc::SIGABRT).unwrap(),
    };

    /// Floating point exception.
    pub const FPE: Self = Self {
        signo: NonZero::new(libc::SIGFPE).unwrap(),
    };

    /// Kill signal.
    pub const KILL: Self = Self {
        signo: NonZero::new(libc::SIGKILL).unwrap(),
    };

    /// Invalid memory reference.
    pub const SEGV: Self = Self {
        signo: NonZero::new(libc::SIGSEGV).unwrap(),
    };

    /// Broken pipe: write to pipe with no readers.
    pub const PIPE: Self = Self {
        signo: NonZero::new(libc::SIGPIPE).unwrap(),
    };

    /// Timer signal from alarm(2).
    pub const ALRM: Self = Self {
        signo: NonZero::new(libc::SIGALRM).unwrap(),
    };

    /// Termination signal.
    pub const TERM: Self = Self {
        signo: NonZero::new(libc::SIGTERM).unwrap(),
    };

    /// User-defined signal 1.
    pub const USR1: Self = Self {
        signo: NonZero::new(libc::SIGUSR1).unwrap(),
    };

    /// User-defined signal 2.
    pub const USR2: Self = Self {
        signo: NonZero::new(libc::SIGUSR2).unwrap(),
    };

    /// Child stopped of terminated.
    pub const CHLD: Self = Self {
        signo: NonZero::new(libc::SIGCHLD).unwrap(),
    };

    /// Continue if stopped.
    pub const CONT: Self = Self {
        signo: NonZero::new(libc::SIGCONT).unwrap(),
    };

    /// Stop process.
    pub const STOP: Self = Self {
        signo: NonZero::new(libc::SIGSTOP).unwrap(),
    };

    /// Stop typed at terminal.
    pub const TSTP: Self = Self {
        signo: NonZero::new(libc::SIGTSTP).unwrap(),
    };

    /// Terminal input for background process.
    pub const TTIN: Self = Self {
        signo: NonZero::new(libc::SIGTTIN).unwrap(),
    };

    /// Terminal output for background process.
    pub const TTOU: Self = Self {
        signo: NonZero::new(libc::SIGTTOU).unwrap(),
    };

    /// Bus error (bad memory access).
    pub const BUS: Self = Self {
        signo: NonZero::new(libc::SIGBUS).unwrap(),
    };

    /// Pollable event (Sys V). Synonym for SIGIO.
    #[cfg(target_os = "linux")]
    pub const POLL: Self = Self {
        signo: NonZero::new(libc::SIGPOLL).unwrap(),
    };

    /// Profiling timer expired.
    pub const PROF: Self = Self {
        signo: NonZero::new(libc::SIGPROF).unwrap(),
    };

    /// Bad argument to routine (SVr4).
    pub const SYS: Self = Self {
        signo: NonZero::new(libc::SIGSYS).unwrap(),
    };

    /// Trace/breakpoint trap.
    pub const TRAP: Self = Self {
        signo: NonZero::new(libc::SIGTRAP).unwrap(),
    };

    /// Urgent condition on socket (4.2BSD).
    pub const URG: Self = Self {
        signo: NonZero::new(libc::SIGURG).unwrap(),
    };

    /// Virtual alarm clock (4.2BSD).
    pub const VTALRM: Self = Self {
        signo: NonZero::new(libc::SIGVTALRM).unwrap(),
    };

    /// CPU time limit exceeded (4.2BSD).
    pub const XCPU: Self = Self {
        signo: NonZero::new(libc::SIGXCPU).unwrap(),
    };

    /// File size limit exceeded (4.2BSD).
    pub const XFSZ: Self = Self {
        signo: NonZero::new(libc::SIGXFSZ).unwrap(),
    };
}

impl Into<i32> for Signal {
    fn into(self) -> i32 {
        self.signo.get()
    }
}

fn sigemptyset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigemptyset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

fn sigfillset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigfillset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

fn sigaddset(mask: &mut libc::sigset_t, sig: Signal) -> Result<()> {
    unsafe {
        match libc::sigaddset(mask, sig.into()) {
            -1 => Err(OsError::last()),
            _ => Ok(()),
        }
    }
}

fn sigprocmask(how: i32, set: &libc::sigset_t) -> Result<libc::sigset_t> {
    let mut oset = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        match libc::sigprocmask(how, set, oset.as_mut_ptr()) {
            -1 => Err(OsError::last()),
            _ => Ok(oset.assume_init()),
        }
    }
}

#[cfg(target_os = "linux")]
mod ffi {
    use super::{
        AsyncSignalSet, Result, Signal, SignalSet, sigaddset, sigemptyset, sigfillset, sigprocmask,
    };
    use crate::IoContext;
    use crate::error::OsError;
    use crate::exec::async_socket::AsyncSocket;
    use crate::ffi::socket::{Fd, Socket};
    use std::cell::Cell;
    use std::num::NonZero;
    use std::time::Instant;

    fn signalfd(mask: &libc::sigset_t) -> Result<Socket> {
        unsafe {
            match libc::signalfd(-1, mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) {
                -1 => Err(OsError::last()),
                sfd => Ok(Socket::from_raw_fd(Fd::new_unchecked(sfd))),
            }
        }
    }

    pub struct SignalSetBuilder<'a> {
        ctx: &'a IoContext,
        set: libc::sigset_t,
    }

    impl<'a> SignalSetBuilder<'a> {
        pub(super) fn new(ctx: &'a IoContext) -> Self {
            Self {
                ctx: ctx,
                set: sigemptyset(),
            }
        }

        pub fn add(mut self, signal: Signal) -> Result<Self> {
            sigaddset(&mut self.set, signal)?;
            Ok(self)
        }

        pub fn any(mut self) -> Self {
            self.set = sigfillset();
            self
        }

        pub fn listen(self) -> Result<SignalSet> {
            sigprocmask(libc::SIG_BLOCK, &self.set)?;
            let sfd = signalfd(&self.set)?;
            Ok(SignalSet {
                ctx: self.ctx.clone(),
                sfd: sfd,
                cto: Cell::new(None),
            })
        }

        pub fn listen_async(self) -> Result<AsyncSignalSet> {
            let ss = self.listen()?;
            Ok(ss.into())
        }
    }

    pub fn nb_signal_read(soc: &Socket) -> Result<Signal> {
        let ssi = soc.as_fd().read_data::<libc::signalfd_siginfo>()?;
        Ok(Signal {
            signo: NonZero::new(ssi.ssi_signo as i32).unwrap(),
        })
    }

    pub fn signal_read(soc: &Socket, ctx: &IoContext, time: Option<Instant>) -> Result<Signal> {
        loop {
            match soc.wait_for_readable(time) {
                Ok(()) => loop {
                    match nb_signal_read(soc) {
                        Ok(sig) => return Ok(sig),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if ctx.is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if ctx.is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub async fn async_signal_read(soc: &AsyncSocket) -> Result<Signal> {
        loop {
            match soc.wait_for_readable().await {
                Ok(()) => loop {
                    match nb_signal_read(soc.as_socket()) {
                        Ok(sig) => return Ok(sig),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if soc.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if soc.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}

pub use self::ffi::SignalSetBuilder;

pub struct SignalSet {
    ctx: IoContext,
    sfd: Socket,
    cto: Cell<Option<Instant>>,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> SignalSetBuilder {
        SignalSetBuilder::new(ctx)
    }

    pub fn close(self) -> Result<()> {
        self.sfd.close()
    }

    pub fn expires_at(&self, cto: Instant) {
        self.cto.set(Some(cto))
    }

    pub fn expires_from_now(&self, cto: Duration) {
        self.expires_at(Instant::now() + cto)
    }

    pub fn nb_wait(&self) -> Result<Signal> {
        #[cfg(target_os = "linux")]
        ffi::nb_signal_read(&self.sfd)
    }

    pub fn wait(&self) -> Result<Signal> {
        #[cfg(target_os = "linux")]
        ffi::signal_read(&self.sfd, &self.ctx, self.cto.get())
    }
}

pub struct AsyncSignalSet {
    sfd: AsyncSocket,
}

impl AsyncSignalSet {
    pub fn expires_at(&self, time: Instant) {
        self.sfd.update_schedule(time)
    }

    pub fn expires_from_now(&self, time: Duration) {
        self.expires_at(Instant::now() + time)
    }

    pub fn nb_wait(&self) -> Result<Signal> {
        #[cfg(target_os = "linux")]
        ffi::nb_signal_read(self.sfd.as_socket())
    }

    pub async fn async_wait(&self) -> Result<Signal> {
        #[cfg(target_os = "linux")]
        ffi::async_signal_read(&self.sfd).await
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(sfd: SignalSet) -> AsyncSignalSet {
        Self {
            sfd: AsyncSocket::new(sfd.ctx, sfd.sfd),
        }
    }
}
