use crate::core::{Event, IoContext};
use crate::error::{OsError, Result};
use crate::primitive::{Fd, Socket, Timeout};
use crate::socket::AsyncSocket;
use std::mem::MaybeUninit;
use std::time::Duration;
use std::{ptr, slice};

pub use crate::primitive::Signal;

fn sigemptyset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigemptyset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

fn sigaddset(mask: &mut libc::sigset_t, sig: Signal) {
    unsafe {
        libc::sigaddset(mask, sig.number());
    }
}

fn sigdelset(mask: &mut libc::sigset_t, sig: Signal) {
    unsafe {
        libc::sigdelset(mask, sig.number());
    }
}

fn sigmaskget() -> Result<libc::sigset_t> {
    let mut oldset = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        match libc::pthread_sigmask(libc::SIG_BLOCK, ptr::null(), oldset.as_mut_ptr()) {
            -1 => Err(OsError::last()),
            _ => Ok(oldset.assume_init()),
        }
    }
}

fn sigmaskset(how: i32, set: &libc::sigset_t) -> Result<()> {
    unsafe {
        match libc::pthread_sigmask(how, set, ptr::null_mut()) {
            -1 => Err(OsError::last()),
            _ => Ok(()),
        }
    }
}

struct SignalSetGuard(libc::sigset_t);

impl Drop for SignalSetGuard {
    fn drop(&mut self) {
        let _ = sigmaskset(libc::SIG_SETMASK, &self.0);
    }
}


pub struct SignalSet {
    ctx: IoContext,
    #[cfg(target_os = "linux")]
    sfd: Socket,
    #[cfg(target_os = "macos")]
    set: libc::sigset_t,
    _set: SignalSetGuard,
    t: Timeout,
}


pub struct AsyncSignalSet {
    #[cfg(target_os = "linux")]
    sfd: AsyncSocket,
    #[cfg(target_os = "macos")]
    ev: (IoContext, Event),
    _set: SignalSetGuard,
    t: Timeout,
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    fn signalfd(mask: &libc::sigset_t) -> crate::error::Result<Fd> {
        unsafe {
            match libc::signalfd(-1, mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) {
                -1 => Err(OsError::last()),
                sfd => Ok(Fd::from_raw_fd(sfd)),
            }
        }
    }

    fn signalfd_(fd: &Fd, mask: &libc::sigset_t) -> crate::error::Result<()> {
        unsafe {
            match libc::signalfd(fd.as_raw_fd(), mask, 0) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
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
            soc.nb_read(buf)?;
            let ssi = ssi.assume_init();
            Ok(Signal::from_signalfd_siginfo(&ssi))
        }
    }

    fn wait(ctx: &IoContext, soc: &Socket, timeout: Timeout) -> Result<Signal> {
        let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
        unsafe {
            let buf = slice::from_raw_parts_mut(
                ssi.as_mut_ptr() as *mut u8,
                size_of::<libc::signalfd_siginfo>(),
            );
            soc.read(ctx, buf, timeout)?;
            let ssi = ssi.assume_init();
            Ok(Signal::from_signalfd_siginfo(&ssi))
        }
    }

    async fn async_wait(soc: &AsyncSocket, timeout: Timeout) -> Result<Signal> {
        let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
        unsafe {
            let buf = slice::from_raw_parts_mut(
                ssi.as_mut_ptr() as *mut u8,
                size_of::<libc::signalfd_siginfo>(),
            );
            soc.async_read(buf, timeout).await?;
            let ssi = ssi.assume_init();
            Ok(Signal::from_signalfd_siginfo(&ssi))
        }
    }

    impl SignalSet {
        pub fn new(ctx: &IoContext) -> Result<SignalSet> {
            let mask = sigmaskget()?;
            let sfd = unsafe { Socket(signalfd(&mask)?) };
            Ok(SignalSet {
                ctx: ctx.clone(),
                sig: sfd,
                _set: SignalSetGuard(mask),
                t: Timeout::INFINITE,
            })
        }

        pub fn with_signals<T>(ctx: &IoContext, signals: T) -> Result<SignalSet>
        where
            T: AsRef<[Signal]>,
        {
            let mut mask = sigmaskget()?;
            for sig in signals.as_ref() {
                sigaddset(&mut mask, *sig);
            }
            sigmaskset(libc::SIG_SETMASK, &mask)?;
            let sfd = unsafe { Socket(signalfd(&mask)?) };
            Ok(SignalSet {
                ctx: ctx.clone(),
                sig: sfd,
                _set: SignalSetGuard(mask),
                t: Timeout::INFINITE,
            })
        }

        pub fn as_ctx(&self) -> &IoContext {
            &self.ctx
        }

        pub fn add(&self, sig: Signal) -> Result<()> {
            let mut mask = sigmaskget()?;
            sigaddset(&mut mask, sig);
            sigmaskset(libc::SIG_SETMASK, &mask)?;
            Ok(signalfd_(&self.sig.0, &mask)?)
        }

        pub fn del(&self, sig: Signal) -> Result<()> {
            let mut mask = sigmaskget()?;
            sigdelset(&mut mask, sig);
            sigmaskset(libc::SIG_SETMASK, &mask)?;
            Ok(signalfd_(&self.sig.0, &mask)?)
        }

        pub fn clear(&self) -> Result<()> {
            let mask = sigemptyset();
            sigmaskset(libc::SIG_SETMASK, &mask)?;
            Ok(signalfd_(&self.sig.0, &mask)?)
        }

        pub const fn set_timeout(&mut self, timeout: Duration) {
            self.t = Timeout::from_duration(timeout)
        }

        pub fn nb_wait(&self) -> Result<Signal> {
            nb_wait(&self.sig)
        }

        pub fn wait(&self) -> Result<Signal> {
            wait(&self.ctx, &self.sig, self.t)
        }
    }

    impl AsyncSignalSet {
        pub fn as_ctx(&self) -> &IoContext {
            self.sig.as_ctx()
        }

        pub fn add(&self, sig: Signal) -> Result<()> {
            let mut mask = sigmaskget()?;
            sigaddset(&mut mask, sig);
            sigmaskset(libc::SIG_SETMASK, &mask)?;
            Ok(signalfd_(&self.sig.as_socket().0, &mask)?)
        }

        pub fn del(&self, sig: Signal) -> Result<()> {
            let mut mask = sigmaskget()?;
            sigdelset(&mut mask, sig);
            sigmaskset(libc::SIG_SETMASK, &mask)?;
            Ok(signalfd_(&self.sig.as_socket().0, &mask)?)
        }

        pub fn clear(&self) -> Result<()> {
            let mask = sigemptyset();
            sigmaskset(libc::SIG_SETMASK, &mask)?;
            Ok(signalfd_(&self.sig.as_socket().0, &mask)?)
        }

        pub const fn set_timeout(&mut self, timeout: Duration) {
            self.t = Timeout::from_duration(timeout)
        }

        pub fn wait(&self) -> Result<Signal> {
            wait(self.as_ctx(), self.sig.as_socket(), self.t)
        }

        pub async fn async_wait(&self) -> Result<Signal> {
            async_wait(&self.sig, self.t).await
        }
    }

    impl From<SignalSet> for AsyncSignalSet {
        fn from(sfd: SignalSet) -> Self {
            Self {
                sfd: AsyncSocket::new(sfd.ctx, sfd.sfd),
                _set: sfd._set,
                t: sfd.t,
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    impl SignalSet {
        pub fn new(ctx: &IoContext) -> Result<SignalSet> {
            let mask = sigmaskget()?;
            Ok(SignalSet {
                ctx: ctx.clone(),
                set: mask,
                _set: SignalSetGuard(mask),
                t: Timeout::INFINITE,
            })
        }

        pub fn with_signals<T>(ctx: &IoContext, signals: T) -> Result<SignalSet>
        where
            T: AsRef<[Signal]>,
        {
            let mut mask = sigmaskget()?;
            for sig in signals.as_ref() {
                sigaddset(&mut mask, *sig);
            }
            sigmaskset(libc::SIG_SETMASK, &mask)?;
            Ok(SignalSet {
                ctx: ctx.clone(),
                set: mask,
                _set: SignalSetGuard(mask),
                t: Timeout::INFINITE,
            })
        }

        pub fn as_ctx(&self) -> &IoContext {
            &self.ctx
        }

        pub fn add(&mut self, sig: Signal) -> Result<()> {
            sigaddset(&mut self.set, sig);
            Ok(())
        }

        pub fn del(&mut self, sig: Signal) -> Result<()> {
            sigdelset(&mut self.set, sig);
            Ok(())
        }

        pub fn clear(&mut self) -> Result<()> {
            self.set = sigemptyset();
            Ok(())
        }

        pub const fn set_timeout(&mut self, timeout: Duration) {
            self.t = Timeout::from_duration(timeout)
        }

        pub fn wait(&self) -> Result<Signal> {
            let mut sig = MaybeUninit::uninit();
            unsafe {
                match libc::sigwait(&self.set, sig.as_mut_ptr()) {
                    -1 => Err(OsError::last()),
                    _ => {
                        let sig = sig.assume_init();
                        Ok(Signal::from_raw(sig))
                    }
                }
            }
        }
    }

    impl AsyncSignalSet {
        pub async fn async_wait(&self) -> Result<Signal> {
            let event = self.ev.1.lock();
            match event.poll_sig(&self.ev.1, self.t).await {
                Ok(sig) => Ok(sig),
                Err(()) => Err(OsError::OPERATION_CANCELED),
            }
        }
    }
}

