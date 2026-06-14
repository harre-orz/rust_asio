use super::{Intr, Scheduler};
use crate::error::OsError;
use crate::primitive::{Deadline, Fd, Signal, Socket, Timeout};
use std::mem;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

enum State {
    Ready(libc::uintptr_t),
    Cancel,
    Queued(Waker),
}

impl State {
    fn ok(&mut self, ident: libc::uintptr_t) -> Option<Waker> {
        let mut state = State::Ready(ident);
        mem::swap(self, &mut state);
        if let State::Queued(waker) = state {
            Some(waker)
        } else {
            None
        }
    }

    fn cancel(&mut self) -> Option<Waker> {
        let mut state = State::Cancel;
        mem::swap(self, &mut state);
        if let State::Queued(waker) = state {
            Some(waker)
        } else {
            None
        }
    }
}

struct Inner {
    readable: State,
    writable: State,
    signaled: State,
}

impl Inner {
    const fn new() -> Self {
        Self {
            readable: State::Ready(0),
            writable: State::Ready(0),
            signaled: State::Ready(0),
        }
    }
}

pub struct WaitForReadable<'a, T> {
    event: &'a Kevent<T>,
    guard: Option<KeventGuard<'a, T>>,
}

impl<'a, T> Future for WaitForReadable<'a, T> {
    type Output = Result<(), ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut guard) = self.guard.take() {
            guard.mutex.readable = State::Queued(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            match event.readable {
                State::Ready(_) => Poll::Ready(Ok(())),
                State::Cancel => Poll::Ready(Err(())),
                State::Queued(_) => Poll::Pending,
            }
        }
    }
}

unsafe impl<'a, T> Send for WaitForReadable<'a, T> {}

unsafe impl<'a, T> Sync for WaitForReadable<'a, T> {}

pub struct WaitForWritable<'a, T> {
    event: &'a Kevent<T>,
    guard: Option<KeventGuard<'a, T>>,
}

unsafe impl<'a, T> Send for WaitForWritable<'a, T> {}

unsafe impl<'a, T> Sync for WaitForWritable<'a, T> {}

impl<'a, T> Future for WaitForWritable<'a, T> {
    type Output = Result<(), ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut guard) = self.guard.take() {
            guard.mutex.writable = State::Queued(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            match event.writable {
                State::Ready(_) => Poll::Ready(Ok(())),
                State::Cancel => Poll::Ready(Err(())),
                State::Queued(_) => Poll::Pending,
            }
        }
    }
}

pub struct WaitForSignaled<'a, T> {
    event: &'a Kevent<T>,
    guard: Option<KeventGuard<'a, T>>,
}

impl<'a, T> Future for WaitForSignaled<'a, T> {
    type Output = Result<Signal, ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut guard) = self.guard.take() {
            guard.mutex.signaled = State::Queued(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            match event.signaled {
                State::Ready(sig) => Poll::Ready(Ok(unsafe { Signal::from_raw(sig as i32) })),
                State::Cancel => Poll::Ready(Err(())),
                State::Queued(_) => Poll::Pending,
            }
        }
    }
}

unsafe impl<'a, T> Send for WaitForSignaled<'a, T> {}

unsafe impl<'a, T> Sync for WaitForSignaled<'a, T> {}

pub(crate) struct KeventGuard<'a, T> {
    mutex: MutexGuard<'a, Inner>,
    event: &'a Kevent<T>,
}

impl<'a, T> KeventGuard<'a, T> {
    pub(crate) fn poll_in(self, t: Timeout) -> WaitForReadable<'a, T> {
        WaitForReadable {
            event: self.event,
            guard: Some(self),
        }
    }

    pub(crate) fn poll_out(self, t: Timeout) -> WaitForWritable<'a, T> {
        WaitForWritable {
            event: self.event,
            guard: Some(self),
        }
    }

    pub(crate) fn poll_sig(self, t: Timeout) -> WaitForSignaled<'a, T> {
        WaitForSignaled {
            event: self.event,
            guard: Some(self),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Kevent<T>(Arc<(Mutex<Inner>, T)>);

impl<T> Kevent<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new((
            Mutex::new(Inner::new()),
            data,
        )))
    }

    pub fn as_data(&self) -> &T {
        &self.0.1
    }

    pub(crate) fn lock(&self) -> KeventGuard<'_, T> {
        KeventGuard {
            mutex: self.0.0.lock().unwrap(),
            event: self,
        }
    }

    pub fn cancel(&self, vec: &mut Vec<Waker>) {
        let mut event = self.0.0.lock().unwrap();
        if let Some(waker) = event.readable.cancel() {
            vec.push(waker);
        }
        if let Some(waker) = event.writable.cancel() {
            vec.push(waker);
        }
        if let Some(waker) = event.signaled.cancel() {
            vec.push(waker);
        }
    }
}

fn kqueue() -> Result<Fd, OsError> {
    unsafe {
        match libc::kqueue() {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

trait AsKeventIdent {
    fn ident(&self) -> libc::uintptr_t;
}

impl AsKeventIdent for Fd {
    fn ident(&self) -> libc::uintptr_t {
        unsafe { self.as_raw_fd() as libc::uintptr_t }
    }
}

impl AsKeventIdent for Signal {
    fn ident(&self) -> libc::uintptr_t {
        self.number() as libc::uintptr_t
    }
}

fn kevent_set<T, U>(data: &T, filter: i16, flags: u16, event: &Kevent<U>) -> libc::kevent
where
    T: AsKeventIdent,
{
    unsafe {
        libc::kevent {
            ident: data.ident(),
            filter: filter,
            flags: flags,
            fflags: 0,
            data: 0,
            udata: Arc::into_raw(event.0.clone()).cast_mut().cast(),
        }
    }
}

fn kevent(
    kq: &Fd,
    changes: &[libc::kevent],
    mut timeout: libc::timespec,
) -> Result<(Box<[libc::kevent]>, usize), OsError> {
    let mut kevents: Box<[MaybeUninit<libc::kevent>]> =
        Box::<[libc::kevent]>::new_uninit_slice(changes.len());
    unsafe {
        match libc::kevent(
            kq.as_raw_fd(),
            changes.as_ptr(),
            changes.len() as libc::c_int,
            kevents[0].as_mut_ptr(),
            kevents.len() as libc::c_int,
            &mut timeout,
        ) {
            -1 => Err(OsError::last()),
            len => Ok((kevents.assume_init(), len as usize)),
        }
    }
}

pub(in super::super) struct Kqueue {
    kq: Fd,
    kevents: Mutex<Vec<libc::kevent>>,
    intr: Intr,
    intr_event: Kevent<()>,
}

impl Kqueue {
    pub fn new() -> Result<Self, OsError> {
        let kq = kqueue()?;
        let intr = Intr::new()?;
        let intr_event = Kevent::new(());
        let mut kevents = Vec::new();
        kevents.push(kevent_set(
            intr.as_fd(),
            libc::EVFILT_READ,
            libc::EV_ADD | libc::EV_ENABLE,
            &intr_event,
        ));
        Ok(Self {
            kq: kq,
            kevents: Mutex::new(kevents),
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn add_socket<T>(&self, soc: &Socket, event: &Kevent<T>) {
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            &soc.0,
            libc::EVFILT_READ,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
        kevents.push(kevent_set(
            &soc.0,
            libc::EVFILT_WRITE,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
    }

    pub(crate) fn del_socket(&self, soc: &Socket) {
        let mut kevents = self.kevents.lock().unwrap();
        let mut i = 0;
        while i < kevents.len() {
            let filter = kevents[i].filter;
            if (filter == libc::EVFILT_READ || filter == libc::EVFILT_WRITE) && kevents[i].ident == soc.0.ident() {
                kevents.remove(i);
            } else {
                i += 1
            }
        }
    }

    pub fn add_signal<T>(&self, sig: Signal, event: &Kevent<T>) {
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            &sig,
            libc::EVFILT_SIGNAL,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
    }

    pub(crate) fn del_signal(&self, sig: Signal) {
        let mut kevents = self.kevents.lock().unwrap();
        let mut i = 0;
        while i < kevents.len() {
            let filter = kevents[i].filter;
            if filter == libc::EVFILT_SIGNAL && kevents[i].ident == sig.ident() {
                kevents.remove(i);
            } else {
                i += 1
            }
        }
    }

    pub fn wake_up_now(&self) {
        self.intr.wake_up_now()
    }

    pub fn poll(&self, scheduler: &Scheduler) -> Poll<OsError> {
        let mut wakers = Vec::new();
        let changes = {
            let kevents = self.kevents.lock().unwrap();
            kevents.clone()
        };
        loop {
            match kevent(&self.kq, changes.as_slice(), self.intr.timeout_kqueue()) {
                Err(OsError::INTERRUPTED) => continue,
                Err(err) => return Poll::Ready(err),
                Ok((kevents, len)) => {
                    let now = Deadline::now();
                    for kev in &kevents[..len] {
                        let event: Kevent<()> = Kevent(unsafe { Arc::from_raw(kev.udata.cast()) });
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event();
                            continue;
                        }
                        if kev.filter == libc::EVFILT_READ {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.readable.ok(kev.ident) {
                                wakers.push(waker);
                            }
                        }
                        if kev.filter == libc::EVFILT_WRITE {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.writable.ok(kev.ident) {
                                wakers.push(waker);
                            }
                        }
                        if (kev.filter == libc::EVFILT_SIGNAL) {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.signaled.ok(kev.ident) {
                                wakers.push(waker);
                            }
                        }
                        scheduler.update_event(&event, now, &mut wakers);
                    }
                    for waker in wakers {
                        waker.wake()
                    }
                    return Poll::Pending;
                }
            }
        }
    }
}
