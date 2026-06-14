use super::{Intr, Scheduler};
use crate::error::OsError;
use crate::primitive::{Deadline, Fd, Signal, Socket, Timeout};
use std::mem;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

enum EventOp {
    Ready(Result<libc::uintptr_t, ()>),
    Pending(Waker),
}

impl EventOp {
    fn result(&mut self, res: Result<libc::uintptr_t, ()>) -> Option<Waker> {
        let mut event = EventOp::Ready(res);
        mem::swap(self, &mut event);
        if let EventOp::Pending(waker) = event {
            Some(waker)
        } else {
            None
        }
    }
}

struct Inner {
    readable: EventOp,
    writable: EventOp,
    signaled: EventOp,
}

impl Inner {
    const fn new() -> Self {
        Self {
            readable: EventOp::Ready(Ok(0)),
            writable: EventOp::Ready(Ok(0)),
            signaled: EventOp::Ready(Ok(0)),
        }
    }
}

pub struct WaitForReadable<'a> {
    event: &'a Kevent,
    guard: Option<KeventGuard<'a>>,
}

impl<'a> Future for WaitForReadable<'a> {
    type Output = Result<(), ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut guard) = self.guard.take() {
            guard.0.readable = EventOp::Pending(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            if let EventOp::Ready(res) = event.readable {
                Poll::Ready(res.map(|_| ()))
            } else {
                Poll::Pending
            }
        }
    }
}

unsafe impl<'a> Send for WaitForReadable<'a> {}

unsafe impl<'a> Sync for WaitForReadable<'a> {}

pub struct WaitForWritable<'a> {
    event: &'a Kevent,
    guard: Option<KeventGuard<'a>>,
}

unsafe impl<'a> Send for WaitForWritable<'a> {}

unsafe impl<'a> Sync for WaitForWritable<'a> {}

impl<'a> Future for WaitForWritable<'a> {
    type Output = Result<(), ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut guard) = self.guard.take() {
            guard.0.writable = EventOp::Pending(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            if let EventOp::Ready(res) = event.writable {
                Poll::Ready(res.map(|_| ()))
            } else {
                Poll::Pending
            }
        }
    }
}

pub struct WaitForSignaled<'a> {
    event: &'a Kevent,
    guard: Option<KeventGuard<'a>>,
}

unsafe impl<'a> Send for WaitForSignaled<'a> {}

unsafe impl<'a> Sync for WaitForSignaled<'a> {}

impl<'a> Future for WaitForSignaled<'a> {
    type Output = Result<Signal, ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut guard) = self.guard.take() {
            guard.0.signaled = EventOp::Pending(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            if let EventOp::Ready(res) = event.signaled {
                Poll::Ready(res.map(|sig| unsafe { Signal::from_raw(sig as i32) }))
            } else {
                Poll::Pending
            }
        }
    }
}

pub(crate) struct KeventGuard<'a>(MutexGuard<'a, Inner>);

impl<'a> KeventGuard<'a> {
    pub(crate) fn poll_in(self, event: &'a Kevent, t: Timeout) -> WaitForReadable<'a> {
        WaitForReadable {
            event: event,
            guard: Some(self),
        }
    }

    pub(crate) fn poll_out(self, event: &'a Kevent, t: Timeout) -> WaitForWritable<'a> {
        WaitForWritable {
            event: event,
            guard: Some(self),
        }
    }

    pub(crate) fn poll_sig(self, event: &'a Kevent, t: Timeout) -> WaitForSignaled<'a> {
        WaitForSignaled {
            event: event,
            guard: Some(self),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Kevent(Arc<(Mutex<Inner>, Option<Socket>)>);

impl Kevent {
    pub fn new(soc: Socket) -> Self {
        Self(Arc::new((
            Mutex::new(Inner {
                readable: EventOp::Ready(Ok(0)),
                writable: EventOp::Ready(Ok(0)),
                signaled: EventOp::Ready(Ok(0)),
            }),
            Some(soc),
        )))
    }

    pub fn as_socket(&self) -> &Socket {
        &self.0.1.as_ref().unwrap()
    }

    pub(crate) fn lock(&self) -> KeventGuard<'_> {
        KeventGuard(self.0.0.lock().unwrap())
    }

    pub fn cancel(&self, vec: &mut Vec<Waker>) {}
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

fn kevent_set<T>(data: &T, filter: i16, flags: u16, event: &Kevent) -> libc::kevent
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
    intr_event: Kevent,
}

impl Kqueue {
    pub fn new() -> Result<Self, OsError> {
        let kq = kqueue()?;
        let (intr, fd) = Intr::new()?;
        let intr_event = Kevent(Arc::new((
            Mutex::new(Inner::new()),
            #[cfg(not(feature = "timerfd"))]
            Some(Socket(fd)),
            #[cfg(feature = "timerfd")]
            None,
        )));
        let mut kevents = Vec::new();
        kevents.push(kevent_set(
            &intr_event.0.0.1,
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

    pub(crate) fn add_socket(&self, event: &Kevent) {
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            &event.0.1.as_ref().unwrap(),
            libc::EVFILT_READ,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
        kevents.push(kevent_set(
            &event.0.0.0,
            libc::EVFILT_WRITE,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
    }

    pub(crate) fn del_socket(&self, event: &Kevent) {
        let mut kevents = self.kevents.lock().unwrap();
        let mut i = 0;
        while i < kevents.len() {
            if kevents[i].ident == event.as_socket().0.ident() {
                kevents.remove(i);
            } else {
                i += 1
            }
        }
    }

    pub fn add_signal(&self, sig: Signal, event: &Kevent) {
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            &sig,
            libc::EVFILT_SIGNAL,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
    }

    pub(super) fn del_signal(&self, sig: Signal, event: &Kevent) {
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            &sig,
            libc::EVFILT_SIGNAL,
            libc::EV_DELETE,
            event,
        ));
    }

    pub fn wake_up_now(&self) {
        self.intr.wake_up_now(&self.intr_event.as_socket().0)
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
                        let event = Kevent(unsafe { Arc::from_raw(kev.udata.cast()) });
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event(&self.intr_event.0.0.0);
                            continue;
                        }
                        if kev.filter == libc::EVFILT_READ {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.readable.result(Ok(0)) {
                                wakers.push(waker);
                            }
                        }
                        if kev.filter == libc::EVFILT_WRITE {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.writable.result(Ok(0)) {
                                wakers.push(waker);
                            }
                        }
                        if (kev.filter == libc::EVFILT_SIGNAL) {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.signaled.result(Ok(kev.ident)) {
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
