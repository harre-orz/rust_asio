/// A list specifying POSIX categories of signal.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Signal(pub(crate) i32);

impl Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    pub const HUP: Self = Self(libc::SIGHUP);

    /// Interrupt from keyboard.
    pub const INT: Self = Self(libc::SIGINT);

    /// Quit from keyboard.
    pub const QUIT: Self = Self(libc::SIGQUIT);

    /// Illegal Instruction.
    pub const ILL: Self = Self(libc::SIGILL);

    /// Abort signal from abort(3)
    pub const ABRT: Self = Self(libc::SIGABRT);

    /// Floating point exception.
    pub const FPE: Self = Self(libc::SIGFPE);

    /// Kill signal.
    pub const KILL: Self = Self(libc::SIGKILL);

    /// Invalid memory reference.
    pub const SEGV: Self = Self(libc::SIGSEGV);

    /// Broken pipe: write to pipe with no readers.
    pub const PIPE: Self = Self(libc::SIGPIPE);

    /// Timer signal from alarm(2).
    pub const ALRM: Self = Self(libc::SIGALRM);

    /// Termination signal.
    pub const TERM: Self = Self(libc::SIGTERM);

    /// User-defined signal 1.
    pub const USR1: Self = Self(libc::SIGUSR1);

    /// User-defined signal 2.
    pub const USR2: Self = Self(libc::SIGUSR2);

    /// Child stopped of terminated.
    pub const CHLD: Self = Self(libc::SIGCHLD);

    /// Continue if stopped.
    pub const CONT: Self = Self(libc::SIGCONT);

    /// Stop process.
    pub const STOP: Self = Self(libc::SIGSTOP);

    /// Stop typed at terminal.
    pub const TSTP: Self = Self(libc::SIGTSTP);

    /// Terminal input for background process.
    pub const TTIN: Self = Self(libc::SIGTTIN);

    /// Terminal output for background process.
    pub const TTOU: Self = Self(libc::SIGTTOU);

    /// Bus error (bad memory access).
    pub const BUS: Self = Self(libc::SIGBUS);

    /// Pollable event (Sys V). Synonym for SIGIO.
    #[cfg(target_os = "linux")]
    pub const POLL: Self = Self(libc::SIGPOLL);

    /// Profiling timer expired.
    pub const PROF: Self = Self(libc::SIGPROF);

    /// Bad argument to routine (SVr4).
    pub const SYS: Self = Self(libc::SIGSYS);

    /// Trace/breakpoint trap.
    pub const TRAP: Self = Self(libc::SIGTRAP);

    /// Urgent condition on socket (4.2BSD).
    pub const URG: Self = Self(libc::SIGURG);

    /// Virtual alarm clock (4.2BSD).
    pub const VTALRM: Self = Self(libc::SIGVTALRM);

    /// CPU time limit exceeded (4.2BSD).
    pub const XCPU: Self = Self(libc::SIGXCPU);

    /// File size limit exceeded (4.2BSD).
    pub const XFSZ: Self = Self(libc::SIGXFSZ);
}

impl Into<i32> for Signal {
    fn into(self) -> i32 {
        self.0 as i32
    }
}
