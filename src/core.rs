use std::num::NonZero;
use crate::error::Result;
use std::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Timeout(libc::c_int);

impl Timeout {
    pub const fn infinite() -> Self {
        Self(-1)
    }

    pub const fn from_duration(timeout: Duration) -> Self {
        let time = timeout.as_millis();
        if time > i32::MAX as u128 {
            Timeout::infinite()
        } else {
            Timeout(time as i32)
        }
    }

    pub const fn into_duration(self) -> Duration {
        let millis = if self.0 == -1 {
            u32::MAX
        } else {
            self.0 as u32
        };
        Duration::from_millis(millis as u64)
    }
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::Fd;
#[cfg(unix)]
pub use self::unix::{Socket};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::Socket;
#[cfg(windows)]
pub(crate) use self::windows::{AsHandle, Handle, WinSockEx};

pub(crate) fn connect<E>(
    ctx: &IoContext,
    soc: &Socket,
    ep: &EndpointRef<E>,
    timeout: Timeout,
) -> Result<()>
where
    E: Endpoint,
{
    loop {
        match soc.connect(ep) {
            Ok(_) => return Ok(()),
            Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                if let Err(err) = soc.poll_out(timeout) {
                    return Err(err);
                }
            }
            Err(OsError::INTERRUPTED) => {
                if ctx.is_stopped() {
                    return Err(OsError::OPERATION_CANCELED);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub(crate) fn accept<E>(ctx: &IoContext, soc: &Socket, timeout: Timeout) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.accept() {
                    Ok(soc) => return Ok(soc),
                    Err(OsError::INTERRUPTED) => {
                        if ctx.is_stopped() {
                            return Err(OsError::OPERATION_CANCELED);
                        }
                    }
                    #[allow(unreachable_patterns)]
                    Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
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

pub(crate) fn write_some(
    ctx: &IoContext,
    soc: &Socket,
    buf: &[u8],
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_out(timeout) {
            Ok(()) => loop {
                match soc.write(buf) {
                    Ok(len) => return Ok(len),
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

pub(crate) fn send(ctx: &IoContext, soc: &Socket, buf: &[u8], timeout: Timeout) -> Result<usize> {
    loop {
        match soc.poll_out(timeout) {
            Ok(()) => loop {
                match soc.send(buf) {
                    Ok(len) => return Ok(len),
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

pub(crate) fn send_to<E>(
    ctx: &IoContext,
    soc: &Socket,
    buf: &[u8],
    ep: &EndpointRef<E>,

    timeout: Timeout,
) -> Result<usize>
where
    E: Endpoint,
{
    loop {
        match soc.poll_out(timeout) {
            Ok(()) => loop {
                match soc.send_to(buf, ep) {
                    Ok(len) => return Ok(len),
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

pub(crate) fn send_msg(
    ctx: &IoContext,
    soc: &Socket,
    mbuf: &mut MsgBuf,
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_out(timeout) {
            Ok(()) => loop {
                match soc.send_msg(mbuf) {
                    Ok(len) => return Ok(len),
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

pub(crate) fn read_some(
    ctx: &IoContext,
    soc: &Socket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.read(buf) {
                    Ok(len) => return Ok(len),
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

pub(crate) fn receive(
    ctx: &IoContext,
    soc: &Socket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.receive(buf) {
                    Ok(len) => return Ok(len),
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

pub(crate) fn receive_from<E>(
    ctx: &IoContext,
    soc: &Socket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<(usize, E)>
where
    E: Endpoint,
{
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.receive_from(buf) {
                    Ok(len) => return Ok(len),
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

pub(crate) fn receive_msg(
    ctx: &IoContext,
    soc: &Socket,
    mbuf: &mut MsgBuf,
    timeout: Timeout,
) -> Result<usize> {
    loop {
        match soc.poll_in(timeout) {
            Ok(()) => loop {
                match soc.receive_msg(mbuf) {
                    Ok(len) => return Ok(len),
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

#[cfg(unix)]
pub(crate) use unix::{
    async_accept, async_connect, async_read_some, async_receive, async_receive_from,
    async_receive_msg, async_send, async_send_msg, async_send_to, async_write_some,
};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::{
    async_accept, async_connect, async_read_some, async_receive, async_receive_from,
    async_receive_msg, async_send, async_send_msg, async_send_to, async_write_some,
};

use crate::buffer::MsgBuf;
use crate::error::OsError;
use crate::socket_base::{Endpoint, EndpointRef};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

mod scheduler;
use self::scheduler::{Deadline, EventScheduler};

#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
mod intr_timerfd;
#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
use self::intr_timerfd::TimerFd as Interrupter;

#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
mod intr_eventfd;
#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
use self::intr_eventfd::EventFd as Interrupter;

#[cfg(any(
    target_os = "macos",
    not(any(windows, feature = "timerfd", feature = "eventfd"))
))]
mod intr_pipe_unix;
#[cfg(any(
    target_os = "macos",
    not(any(windows, feature = "timerfd", feature = "eventfd"))
))]
use self::intr_pipe_unix::Pipe as Interrupter;

#[cfg(windows)]
mod intr_pipe_win;
#[cfg(windows)]
use self::intr_pipe_win::Pipe as Interrupter;

#[cfg(target_os = "linux")]
mod poll_epoll;
#[cfg(target_os = "linux")]
use self::poll_epoll::{Epoll as Reactor, Event};

#[cfg(target_os = "macos")]
mod poll_kqueue;
#[cfg(target_os = "macos")]
use self::poll_kqueue::{Event, Kqueue as Reactor};

#[cfg(windows)]
mod poll_iocp;
#[cfg(windows)]
use self::poll_iocp::{Event, Iocp as Reactor};

mod socket;

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


pub(crate) use self::socket::AsyncSocket;

struct Inner {
    #[cfg(windows)]
    winsock: crate::socket::WinSockEx,
    reactor: Reactor,
    waker: Mutex<Option<Waker>>,
    scheduler: EventScheduler,
    stop: AtomicBool,
}

struct FutureRun(Arc<Inner>);

impl Future for FutureRun {
    type Output = crate::error::Result<()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if self.0.scheduler.pending_count() == 0 {
            return Poll::Ready(Ok(()));
        }

        if self.0.stop.load(Ordering::Relaxed) {
            let mut vec = Vec::new();
            self.0.scheduler.cancel_all_events(&mut vec);
            for waker in vec {
                waker.wake();
            }
            Poll::Pending
        } else {
            match self.0.reactor.poll(&self.0.scheduler) {
                Poll::Pending => {
                    let mut waker = self.0.waker.lock().unwrap();
                    *waker = Some(ctx.waker().clone());
                    Poll::Pending
                }
                Poll::Ready(err) => Poll::Ready(Err(err)),
            }
        }
    }
}

#[derive(Clone)]
pub struct IoContext {
    inner: Arc<Inner>,
}

impl IoContext {
    pub fn new() -> crate::error::Result<Self> {
        #[cfg(windows)]
        let winsock = crate::socket::WinSockEx::new()?;

        let reactor = Reactor::new()?;
        Ok(Self {
            inner: Arc::new(Inner {
                #[cfg(windows)]
                winsock: winsock,
                waker: Mutex::new(None),
                reactor: reactor,
                scheduler: EventScheduler::new(),
                stop: AtomicBool::new(false),
            }),
        })
    }

    #[cfg(windows)]
    pub fn winsock(&self) -> &crate::socket::WinSockEx {
        &self.inner.winsock
    }

    pub fn is_stopped(&self) -> bool {
        self.inner.stop.load(Ordering::SeqCst)
    }

    pub fn stop(&self) -> bool {
        match self
            .inner
            .stop
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => {
                self.inner.reactor.intr.wake_up_now();
                true
            }
            Err(_) => false,
        }
    }

    pub async fn run(&self) -> crate::error::Result<()> {
        FutureRun(self.inner.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run() {
        let ctx = IoContext::new().unwrap();
        assert_eq!(ctx.is_stopped(), false);
        assert_eq!(ctx.run().await, Ok(()));
        assert_eq!(ctx.is_stopped(), false);
    }

    #[tokio::test]
    async fn stop() {
        let ctx = IoContext::new().unwrap();
        assert_eq!(ctx.is_stopped(), false);
        assert_eq!(ctx.stop(), true);
        assert_eq!(ctx.is_stopped(), true);
        assert_eq!(ctx.run().await, Ok(()));
        //assert_eq!(ctx.is_stopped(), false);
    }

    #[test]
    fn stop2() {
        let ctx = IoContext::new().unwrap();
        assert_eq!(ctx.is_stopped(), false);
        assert_eq!(ctx.stop(), true);
        assert_eq!(ctx.is_stopped(), true);
    }
}
