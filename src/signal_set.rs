// use crate::error::OsError;
// use crate::executor::{AsyncSocket, IoContext};
// use crate::socket::ffi::{self, Socket};
// use crate::ops;
// use std::cell::Cell;
// use std::num::NonZero;
// use std::time::{Duration, Instant};
// 
// type Result<T> = std::result::Result<T, OsError>;
// 
// use std::num::NonZero;
// 
// /// A list specifying POSIX categories of signal.
// #[derive(Clone, Copy, Eq, PartialEq, Debug)]
// pub struct Signal {
//     signo: NonZero<i32>,
// }
// 
// #[cfg(unix)]
// mod unix {
//     use std::mem;
//     use std::mem::MaybeUninit;
//     use super::*;
// 
//     impl Signal {
//         /// Hangup detected on controlling terminal or death of controlling process.
//         pub const HUP: Self = Self {
//             signo: NonZero::new(libc::SIGHUP).unwrap(),
//         };
// 
//         /// Interrupt from keyboard.
//         pub const INT: Self = Self {
//             signo: NonZero::new(libc::SIGINT).unwrap(),
//         };
// 
//         /// Quit from keyboard.
//         pub const QUIT: Self = Self {
//             signo: NonZero::new(libc::SIGQUIT).unwrap(),
//         };
// 
//         /// Illegal Instruction.
//         pub const ILL: Self = Self {
//             signo: NonZero::new(libc::SIGILL).unwrap(),
//         };
// 
//         /// Abort signal from abort(3)
//         pub const ABRT: Self = Self {
//             signo: NonZero::new(libc::SIGABRT).unwrap(),
//         };
// 
//         /// Floating point exception.
//         pub const FPE: Self = Self {
//             signo: NonZero::new(libc::SIGFPE).unwrap(),
//         };
// 
//         /// Kill signal.
//         pub const KILL: Self = Self {
//             signo: NonZero::new(libc::SIGKILL).unwrap(),
//         };
// 
//         /// Invalid memory reference.
//         pub const SEGV: Self = Self {
//             signo: NonZero::new(libc::SIGSEGV).unwrap(),
//         };
// 
//         /// Broken pipe: write to pipe with no readers.
//         pub const PIPE: Self = Self {
//             signo: NonZero::new(libc::SIGPIPE).unwrap(),
//         };
// 
//         /// Timer signal from alarm(2).
//         pub const ALRM: Self = Self {
//             signo: NonZero::new(libc::SIGALRM).unwrap(),
//         };
// 
//         /// Termination signal.
//         pub const TERM: Self = Self {
//             signo: NonZero::new(libc::SIGTERM).unwrap(),
//         };
// 
//         /// User-defined signal 1.
//         pub const USR1: Self = Self {
//             signo: NonZero::new(libc::SIGUSR1).unwrap(),
//         };
// 
//         /// User-defined signal 2.
//         pub const USR2: Self = Self {
//             signo: NonZero::new(libc::SIGUSR2).unwrap(),
//         };
// 
//         /// Child stopped of terminated.
//         pub const CHLD: Self = Self {
//             signo: NonZero::new(libc::SIGCHLD).unwrap(),
//         };
// 
//         /// Continue if stopped.
//         pub const CONT: Self = Self {
//             signo: NonZero::new(libc::SIGCONT).unwrap(),
//         };
// 
//         /// Stop process.
//         pub const STOP: Self = Self {
//             signo: NonZero::new(libc::SIGSTOP).unwrap(),
//         };
// 
//         /// Stop typed at terminal.
//         pub const TSTP: Self = Self {
//             signo: NonZero::new(libc::SIGTSTP).unwrap(),
//         };
// 
//         /// Terminal input for background process.
//         pub const TTIN: Self = Self {
//             signo: NonZero::new(libc::SIGTTIN).unwrap(),
//         };
// 
//         /// Terminal output for background process.
//         pub const TTOU: Self = Self {
//             signo: NonZero::new(libc::SIGTTOU).unwrap(),
//         };
// 
//         /// Bus error (bad memory access).
//         pub const BUS: Self = Self {
//             signo: NonZero::new(libc::SIGBUS).unwrap(),
//         };
// 
//         /// Pollable event (Sys V). Synonym for SIGIO.
//         #[cfg(target_os = "linux")]
//         pub const POLL: Self = Self {
//             signo: NonZero::new(libc::SIGPOLL).unwrap(),
//         };
// 
//         /// Profiling timer expired.
//         pub const PROF: Self = Self {
//             signo: NonZero::new(libc::SIGPROF).unwrap(),
//         };
// 
//         /// Bad argument to routine (SVr4).
//         pub const SYS: Self = Self {
//             signo: NonZero::new(libc::SIGSYS).unwrap(),
//         };
// 
//         /// Trace/breakpoint trap.
//         pub const TRAP: Self = Self {
//             signo: NonZero::new(libc::SIGTRAP).unwrap(),
//         };
// 
//         /// Urgent condition on socket (4.2BSD).
//         pub const URG: Self = Self {
//             signo: NonZero::new(libc::SIGURG).unwrap(),
//         };
// 
//         /// Virtual alarm clock (4.2BSD).
//         pub const VTALRM: Self = Self {
//             signo: NonZero::new(libc::SIGVTALRM).unwrap(),
//         };
// 
//         /// CPU time limit exceeded (4.2BSD).
//         pub const XCPU: Self = Self {
//             signo: NonZero::new(libc::SIGXCPU).unwrap(),
//         };
// 
//         /// File size limit exceeded (4.2BSD).
//         pub const XFSZ: Self = Self {
//             signo: NonZero::new(libc::SIGXFSZ).unwrap(),
//         };
//     }
// 
//     impl Into<i32> for Signal {
//         fn into(self) -> i32 {
//             self.signo.get()
//         }
//     }
// 
//     pub fn sigemptyset() -> libc::sigset_t {
//         let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
//         unsafe {
//             libc::sigemptyset(mask.as_mut_ptr());
//             mask.assume_init()
//         }
//     }
// 
//     pub fn sigfillset() -> libc::sigset_t {
//         let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
//         unsafe {
//             libc::sigfillset(mask.as_mut_ptr());
//             mask.assume_init()
//         }
//     }
// 
//     pub fn sigaddset(mask: &mut libc::sigset_t, sig: Signal) -> Result<()> {
//         match unsafe { libc::sigaddset(mask, sig.into()) } {
//             -1 => Err(unsafe { OsError::last() }),
//             0 => Ok(()),
//             _ => unreachable!(),
//         }
//     }
// 
//     pub fn sigprocmask(how: i32, set: &libc::sigset_t) -> Result<libc::sigset_t> {
//         let mut oset = MaybeUninit::<libc::sigset_t>::uninit();
//         match unsafe { libc::sigprocmask(how, set, oset.as_mut_ptr()) } {
//             -1 => Err(unsafe { OsError::last() }),
//             0 => Ok(unsafe { oset.assume_init() }),
//             _ => unreachable!(),
//         }
//     }
// 
//     #[cfg(target_os = "linux")]
//     pub fn signal_read(soc: &Socket) -> Result<Signal> {
//         let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
//         const LEN: isize = mem::size_of::<libc::signalfd_siginfo>() as isize;
//         match unsafe {
//             libc::read(
//                 soc.as_raw_fd(),
//                 ssi.as_mut_ptr().cast(),
//                 mem::size_of_val(&ssi),
//             )
//         } {
//             -1 => Err(unsafe { OsError::last() }),
//             0 => Err(OsError::CONNECTION_ABORTED),
//             LEN => unsafe {
//                 let ssi = ssi.assume_init();
//                 Ok(Signal {
//                     signo: NonZero::new_unchecked(ssi.ssi_signo as i32),
//                 })
//             },
//             _ => unreachable!(),
//         }
//     }
// }
// 
// pub struct SignalSetBuilder<'a> {
//     ctx: &'a IoContext,
//     set: libc::sigset_t,
// }
// 
// impl<'a> SignalSetBuilder<'a> {
//     pub fn add(mut self, signal: Signal) -> Result<Self> {
//         ffi::sigaddset(&mut self.set, signal)?;
//         Ok(self)
//     }
// 
//     pub fn any(mut self) -> Self {
//         self.set = ffi::sigfillset();
//         self
//     }
// 
//     #[cfg(target_os = "linux")]
//     pub fn listen(self) -> Result<SignalSet> {
//         ffi::sigprocmask(libc::SIG_BLOCK, &self.set)?;
//         let sfd = ffi::signalfd(&self.set)?;
//         Ok(SignalSet {
//             ctx: self.ctx.clone(),
//             sfd: sfd,
//             exp: Cell::new(None),
//         })
//     }
// 
//     #[cfg(target_os = "linux")]
//     pub fn listen_async(self) -> Result<AsyncSignalSet> {
//         let ss = self.listen()?;
//         Ok(ss.into())
//     }
// }
// 
// pub struct SignalSet {
//     ctx: IoContext,
//     sfd: Socket,
//     exp: Cell<Option<Instant>>,
// }
// 
// impl SignalSet {
//     pub fn new(ctx: &IoContext) -> SignalSetBuilder {
//         SignalSetBuilder {
//             ctx: ctx,
//             set: ffi::sigemptyset(),
//         }
//     }
// 
//     pub fn close(self) -> Result<()> {
//         ffi::close(self.sfd)
//     }
// 
//     pub fn expires_at(&self, time: Instant) {
//         self.exp.set(Some(time))
//     }
// 
//     pub fn expires_from_now(&self, time: Duration) {
//         self.expires_at(Instant::now() + time)
//     }
// 
//     #[cfg(target_os = "linux")]
//     pub fn nb_wait(&self) -> Result<Signal> {
//         ffi::signal_read(&self.sfd)
//     }
// 
//     #[cfg(target_os = "linux")]
//     pub fn wait(&self) -> Result<Signal> {
//         ops::signal_read(&self.sfd, &self.ctx, self.exp.get())
//     }
// }
// 
// pub struct AsyncSignalSet {
//     sfd: AsyncSocket,
// }
// 
// impl AsyncSignalSet {
//     pub fn expires_at(&self, time: Instant) {
//         self.sfd.update_schedule(time)
//     }
// 
//     pub fn expires_from_now(&self, time: Duration) {
//         self.expires_at(Instant::now() + time)
//     }
// 
//     #[cfg(target_os = "linux")]
//     pub fn nb_wait(&self) -> Result<Signal> {
//         ffi::signal_read(self.sfd.as_socket())
//     }
// 
//     #[cfg(target_os = "linux")]
//     pub async fn async_wait(&self) -> Result<Signal> {
//         ops::async_signal_read(&self.sfd).await
//     }
// }
// 
// impl From<SignalSet> for AsyncSignalSet {
//     fn from(sfd: SignalSet) -> AsyncSignalSet {
//         Self {
//             sfd: AsyncSocket::new(sfd.ctx, sfd.sfd),
//         }
//     }
// }
