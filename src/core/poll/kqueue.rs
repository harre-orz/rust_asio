use std::cmp::Ordering;
use super::{Intr, Scheduler, Deadline};
use crate::error::OsError;
use crate::primitive::{Fd, Signal, Socket, Timeout};
use std::mem;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr;
use std::sync::{Mutex, MutexGuard};
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

pub(crate) struct Kevent {
    inner: Mutex<Inner>,
    pub(crate) deadline: Deadline,
}

impl Kevent {
    pub fn new<T>(data: T) -> Pin<Box<(Kevent, T)>> {
        Box::pin((
            Self {
                inner: Mutex::new(Inner {
                    readable: State::Ready(0),
                    writable: State::Ready(0),
                    signaled: State::Ready(0),
                }),
                deadline: Deadline::UNINIT,
            },
            data,
        ))
    }
}


impl PartialEq for Kevent {
    fn eq(&self, other: &Self) -> bool {
        ptr::addr_eq(self, other)
    }
}

impl Eq for Kevent {
}

impl PartialOrd for Kevent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Kevent {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.deadline.cmp(&other.deadline) {
            Ordering::Equal => {
                let l = ptr::from_ref(self) as usize;
                let r = ptr::from_ref(other) as usize;
                l.cmp(&r)
            },
            cmp => cmp,
        }
    }
}

pub struct WaitForReadable<'a, 'b> {
    inner: &'a super::super::Inner,
    event: &'b Kevent,
    mutex: Option<MutexGuard<'b, Inner>>,
}

impl<'a, 'b> Future for WaitForReadable<'a, 'b> {
    type Output = Result<(), ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut mutex) = self.mutex.take() {
            mutex.readable = State::Queued(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.inner.lock().unwrap();
            match event.readable {
                State::Ready(_) => Poll::Ready(Ok(())),
                State::Cancel => Poll::Ready(Err(())),
                State::Queued(_) => Poll::Pending,
            }
        }
    }
}

unsafe impl<'a, 'b> Send for WaitForReadable<'a, 'b> {}

unsafe impl<'a, 'b> Sync for WaitForReadable<'a, 'b> {}

pub struct WaitForWritable<'a, 'b> {
    inner: &'a super::super::Inner,
    event: &'b Kevent,
    mutex: Option<MutexGuard<'b, Inner>>,
}

unsafe impl<'a, 'b> Send for WaitForWritable<'a, 'b> {}

unsafe impl<'a, 'b> Sync for WaitForWritable<'a, 'b> {}

impl<'a, 'b> Future for WaitForWritable<'a, 'b> {
    type Output = Result<(), ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut mutex) = self.mutex.take() {
            mutex.writable = State::Queued(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.inner.lock().unwrap();
            match event.writable {
                State::Ready(_) => Poll::Ready(Ok(())),
                State::Cancel => Poll::Ready(Err(())),
                State::Queued(_) => Poll::Pending,
            }
        }
    }
}

pub struct WaitForSignaled<'a, 'b> {
    inner: &'a super::super::Inner,
    event: &'b Kevent,
    mutex: Option<MutexGuard<'b, Inner>>,
}

impl<'a, 'b> Future for WaitForSignaled<'a, 'b> {
    type Output = Result<Signal, ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut mutex) = self.mutex.take() {
            mutex.signaled = State::Queued(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.inner.lock().unwrap();
            match event.signaled {
                State::Ready(sig) => Poll::Ready(Ok(unsafe { Signal::from_raw(sig as i32) })),
                State::Cancel => Poll::Ready(Err(())),
                State::Queued(_) => Poll::Pending,
            }
        }
    }
}

unsafe impl<'a, 'b> Send for WaitForSignaled<'a, 'b> {}

unsafe impl<'a, 'b> Sync for WaitForSignaled<'a, 'b> {}

pub(crate) struct KeventGuard<'a, 'b> {
    inner: &'a super::super::Inner,
    event: &'b Kevent,
    mutex: MutexGuard<'b, Inner>,
}

impl<'a, 'b> KeventGuard<'a, 'b> {
    pub(in super::super) fn lock(inner: &'a super::super::Inner, event: &'b Kevent) -> Self {
        Self {
            inner: inner,
            event: event,
            mutex: event.inner.lock().unwrap(),
        }
    }

    pub fn poll_in(self, t: Timeout) -> WaitForReadable<'a, 'b> {
        WaitForReadable {
            inner: self.inner,
            event: self.event,
            mutex: Some(self.mutex),
        }
    }

    pub fn poll_out(self, t: Timeout) -> WaitForWritable<'a, 'b> {
        WaitForWritable {
            inner: self.inner,
            event: self.event,
            mutex: Some(self.mutex),
        }
    }

    pub fn poll_sig(self, t: Timeout) -> WaitForSignaled<'a, 'b> {
        WaitForSignaled {
            inner: self.inner,
            event: self.event,
            mutex: Some(self.mutex),
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

fn kevent_set(ident: libc::uintptr_t, filter: i16, flags: u16, event: &Kevent) -> libc::kevent
{
    unsafe {
        libc::kevent {
            ident: ident,
            filter: filter,
            flags: flags,
            fflags: 0,
            data: 0,
            udata: ptr::from_ref(event).cast_mut().cast(),
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
    intr_event: Pin<Box<(Kevent, ())>>,
}

impl Kqueue {
    pub fn new() -> Result<Self, OsError> {
        let kq = kqueue()?;
        let intr = Intr::new()?;
        let intr_event = Kevent::new(());
        let mut kevents = Vec::new();
        #[cfg(feature = "ktimer")]
        kevents.push(kevent_set(
            1,
            libc::EVFILT_TIMER,
            libc::EV_ADD,
            &intr_event.0,
        ));
        #[cfg(not(feature = "ktimer"))]
        kevents.push(kevent_set(
            intr.as_fd(),
            libc::EVFILT_READ,
            libc::EV_ADD | libc::EV_ENABLE,
            &intr_event.0,
        ));
        Ok(Self {
            kq: kq,
            kevents: Mutex::new(kevents),
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn add_socket(&self, soc: &Socket, event: &Kevent) {
        let ident = unsafe { soc.0.as_raw_fd() as libc::uintptr_t };
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            ident,
            libc::EVFILT_READ,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
        kevents.push(kevent_set(
            ident,
            libc::EVFILT_WRITE,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
    }

    pub(crate) fn del_socket(&self, soc: &Socket) {
        let ident = unsafe { soc.0.as_raw_fd() as libc::uintptr_t };
        let mut kevents = self.kevents.lock().unwrap();
        let mut i = 0;
        while i < kevents.len() {
            let filter = kevents[i].filter;
            if (filter == libc::EVFILT_READ || filter == libc::EVFILT_WRITE)
                && kevents[i].ident == ident
            {
                kevents.remove(i);
            } else {
                i += 1
            }
        }
    }

    pub fn add_signal(&self, sig: Signal, event: &Kevent) {
        let ident = sig.number() as libc::uintptr_t;
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            ident,
            libc::EVFILT_SIGNAL,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
    }

    pub(crate) fn del_signal(&self, sig: Signal) {
        let ident = sig.number() as libc::uintptr_t;
        let mut kevents = self.kevents.lock().unwrap();
        let mut i = 0;
        while i < kevents.len() {
            let filter = kevents[i].filter;
            if filter == libc::EVFILT_SIGNAL && kevents[i].ident == ident {
                kevents.remove(i);
            } else {
                i += 1
            }
        }
    }

    pub fn wake_up_now(&self) {
        self.intr.wake_up_now()
    }

    pub(crate) fn cancel_all_events(&self, scheduler: &Scheduler) {}

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
                    for kev in &kevents[..len] {
                        let event: &Mutex<Inner> = unsafe { &*(kev.udata.cast()) };
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event();
                            continue;
                        }
                        if kev.filter == libc::EVFILT_READ {
                            let mut event = event.lock().unwrap();
                            if let Some(waker) = event.readable.ok(kev.ident) {
                                wakers.push(waker);
                            }
                        }
                        if kev.filter == libc::EVFILT_WRITE {
                            let mut event = event.lock().unwrap();
                            if let Some(waker) = event.writable.ok(kev.ident) {
                                wakers.push(waker);
                            }
                        }
                        if (kev.filter == libc::EVFILT_SIGNAL) {
                            let mut event = event.lock().unwrap();
                            if let Some(waker) = event.signaled.ok(kev.ident) {
                                wakers.push(waker);
                            }
                        }
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
