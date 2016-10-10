use std::io;
use {IoObject, IoService, Handler};
use io_service::{IoActor};
use backbone::{RawFd, AsRawFd, AsIoActor};
use backbone::ops::{cancel_io};
use backbone::signalfd::{sigset_t, signalfd_read, signalfd_async_read,
                         signalfd_init, signalfd_add, signalfd_del, signalfd_reset};

pub use backbone::signalfd::{Signal, raise};

/// Provides a signal.
pub struct SignalSet {
    io: IoActor,
    mask: sigset_t,
}

impl SignalSet {
    pub fn new<T: IoObject>(io: &T) -> io::Result<SignalSet> {
        let (fd, mask) = try!(signalfd_init());
        Ok(SignalSet {
            io: IoActor::new(io, fd),
            mask: mask,
        })
    }

    pub fn add(&mut self, signal: Signal) -> io::Result<()> {
        signalfd_add(&self.io, &mut self.mask, signal)
    }

    pub fn async_wait<F: Handler<Signal>>(&self, handler: F) {
        signalfd_async_read(self, handler)
    }

    pub fn cancel(&self) {
        cancel_io(self)
    }

    pub fn clear(&mut self) -> io::Result<()> {
        signalfd_reset(&self.io, &mut self.mask)
    }

    pub fn remove(&mut self, signal: Signal) -> io::Result<()> {
        signalfd_del(&self.io, &mut self.mask, signal)
    }

    pub fn wait(&self) -> io::Result<Signal> {
        signalfd_read(self)
    }
}

impl IoObject for SignalSet {
    fn io_service(&self) -> &IoService {
        self.io.io_service()
    }
}

impl AsRawFd for SignalSet {
    fn as_raw_fd(&self) -> RawFd {
        self.io.as_raw_fd()
    }
}

impl AsIoActor for SignalSet {
    fn as_io_actor(&self) -> &IoActor {
        &self.io
    }
}

impl Drop for SignalSet {
    fn drop(&mut self) {
        signalfd_reset(&self.io, &mut self.mask).unwrap();
    }
}

#[test]
fn test_signal_set() {
    use IoService;

    let io = &IoService::new();
    let mut sig = SignalSet::new(io).unwrap();
    sig.add(Signal::SIGHUP).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR1).unwrap();
    sig.remove(Signal::SIGUSR2).unwrap();
}

#[test]
fn test_signal_set_wait() {
    use IoService;

    let io = &IoService::new();
    let mut sig = SignalSet::new(io).unwrap();
    sig.add(Signal::SIGHUP).unwrap();
    sig.add(Signal::SIGUSR1).unwrap();
    raise(Signal::SIGHUP).unwrap();
    raise(Signal::SIGUSR1).unwrap();
    assert_eq!(sig.wait().unwrap(), Signal::SIGHUP);
    assert_eq!(sig.wait().unwrap(), Signal::SIGUSR1);
}
