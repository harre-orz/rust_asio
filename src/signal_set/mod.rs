use ffi::SystemError;
use core::{AsIoContext, IoContext, ThreadIoContext, Perform};
use ops::{Handler, AsyncReadOp};

use std::io;

pub use ffi::Signal;

#[cfg(target_os = "linux")] mod linux;
#[cfg(target_os = "linux")] use self::linux::{SignalImpl, async_wait};

#[cfg(target_os = "macos")] mod macos;
#[cfg(target_os = "macos")] use self::macos::{SignalImpl, async_wait};

pub struct SignalSet {
    pimpl: Box<SignalImpl>,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> io::Result<Self> {
        Ok(SignalSet { pimpl: SignalImpl::signal(ctx)? })
    }

    pub fn add(&self, sig: Signal) -> io::Result<()> {
        Ok(self.pimpl.add(sig)?)
    }

    pub fn async_wait<F>(&self, handler: F) -> F::Output
    where
        F: Handler<Signal, io::Error>,
    {
        async_wait(self, handler)
    }

    pub fn cancel(&self) {
        self.pimpl.cancel()
    }

    pub fn clear(&self) {
        self.pimpl.clear()
    }

    pub fn remove(&self, sig: Signal) -> io::Result<()> {
        Ok(self.pimpl.remove(sig)?)
    }
}

unsafe impl Send for SignalSet {}

unsafe impl Sync for SignalSet {}

unsafe impl AsIoContext for SignalSet {
    fn as_ctx(&self) -> &IoContext {
        self.pimpl.as_ctx()
    }
}

impl AsyncReadOp for SignalSet {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.pimpl.add_read_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.pimpl.next_read_op(this)
    }
}

pub fn raise(sig: Signal) -> io::Result<()> {
    use ffi;

    Ok(ffi::raise(sig)?)
}

#[test]
fn test_signal_set() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    let mut sig = SignalSet::new(ctx).unwrap();
    sig.add(Signal::SIGHUP).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGHUP).unwrap();
}

#[test]
#[should_panic]
fn test_signal_set_dup_add() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    let mut sig = SignalSet::new(ctx).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
}

#[test]
#[should_panic]
fn test_signal_set_dup_del() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    let mut sig = SignalSet::new(ctx).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR1).unwrap();
}

#[test]
fn test_signal_set_wait() {
    use core::IoContext;
    use ops::wrap;
    use std::sync::Arc;
    use std::thread;

    let ctx = &IoContext::new().unwrap();
    let mut sig = Arc::new(SignalSet::new(ctx).unwrap());
    sig.add(Signal::SIGHUP).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.async_wait(wrap(
        |ctx, sig: io::Result<Signal>| {
            assert_eq!(sig.unwrap(), Signal::SIGHUP)
        },
        &sig,
    ));
    sig.async_wait(wrap(
        |ctx, sig: io::Result<Signal>| {
            assert_eq!(sig.unwrap(), Signal::SIGUSR1)
        },
        &sig,
    ));
    raise(Signal::SIGHUP).unwrap();
    raise(Signal::SIGUSR1).unwrap();
    ctx.run();
}
