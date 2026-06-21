use super::{Deadline, Intr, Scheduler};
use crate::error::OsError;
use crate::primitive::{Fd, Signal, Socket, Timeout};
use std::cmp::Ordering;
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

struct Inner {
    readable: State,
    writable: State,
    signaled: State,
}

pub(crate) struct Kevent {
    inner: Mutex<Inner>,
    pub(in super::super) deadline: Deadline,
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

impl Eq for Kevent {}

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
            }
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
            drop(mutex);
            if self.inner.scheduler.add(self.event) {
                self.inner.reactor.wake_up_alarm(&self.event.deadline)
            }
            Poll::Pending
        } else {
            let event = self.event.inner.lock().unwrap();
            match &event.readable {
                State::Ready(_) => {
                    drop(event);
                    self.inner.scheduler.del(self.event);
                    Poll::Ready(Ok(()))
                }
                State::Cancel => {
                    drop(event);
                    self.inner.scheduler.del(self.event);
                    Poll::Ready(Err(()))
                }
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
            drop(mutex);
            if self.inner.scheduler.add(self.event) {
                self.inner.reactor.wake_up_alarm(&self.event.deadline)
            }
            Poll::Pending
        } else {
            let event = self.event.inner.lock().unwrap();
            match event.writable {
                State::Ready(_) => {
                    drop(event);
                    self.inner.scheduler.del(self.event);
                    Poll::Ready(Ok(()))
                }
                State::Cancel => {
                    drop(event);
                    self.inner.scheduler.del(self.event);
                    Poll::Ready(Err(()))
                }
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
            drop(mutex);
            if self.inner.scheduler.add(self.event) {
                self.inner.reactor.wake_up_alarm(&self.event.deadline)
            }
            Poll::Pending
        } else {
            let event = self.event.inner.lock().unwrap();
            match event.signaled {
                State::Ready(sig) => {
                    drop(event);
                    self.inner.scheduler.del(self.event);
                    Poll::Ready(Ok(unsafe { Signal::from_raw(sig as i32) }))
                },
                State::Cancel => {
                    drop(event);
                    self.inner.scheduler.del(self.event);
                    Poll::Ready(Err(()))
                },
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
        self.event.deadline.update(t);
        WaitForReadable {
            inner: self.inner,
            event: self.event,
            mutex: Some(self.mutex),
        }
    }

    pub fn poll_out(self, t: Timeout) -> WaitForWritable<'a, 'b> {
        self.event.deadline.update(t);
        WaitForWritable {
            inner: self.inner,
            event: self.event,
            mutex: Some(self.mutex),
        }
    }

    pub fn poll_sig(self, t: Timeout) -> WaitForSignaled<'a, 'b> {
        self.event.deadline.update(t);
        WaitForSignaled {
            inner: self.inner,
            event: self.event,
            mutex: Some(self.mutex),
        }
    }
}

fn kqueue() -> Result<Fd, OsError> {
    unsafe {
        match libc::kqueue()  {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

pub(crate) struct Kqueue {
    kq: Fd,
    intr: Intr,
    intr_event: Pin<Box<(Kevent, ())>>,
}

impl Kqueue {
    pub fn new() -> Result<Self, OsError> {
        let kq = kqueue()?;
        let intr = Intr::new()?;
        let intr_event = Kevent::new(());
        #[cfg(not(feature = "ktimer"))]
        unsafe {
            libc::kevent(
                kq.as_raw_fd(),
                &libc::kevent {
                    ident: unsafe { intr.as_fd().as_raw_fd() as libc::uintptr_t },
                    filter: libc::EVFILT_READ,
                    flags: libc::EV_ADD | libc::EV_ENABLE,
                    fflags: 0,
                    data: 0,
                    udata: ptr::from_ref(&intr_event).cast_mut().cast()
                }, 1,
                ptr::null_mut(),0,
                ptr::null_mut(),
            );
        }
        Ok(Self {
            kq: kq,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn add_socket(&self, soc: &Socket, event: &Kevent) {
        let ident = unsafe { soc.0.as_raw_fd() as libc::uintptr_t };
        let kevents: [libc::kevent; 2] = [
            libc::kevent {
                ident: ident,
                filter: libc::EVFILT_READ,
                flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
                fflags: 0,
                data: 0,
                udata: ptr::from_ref(event).cast_mut().cast()
            },
            libc::kevent {
                ident: ident,
                filter: libc::EVFILT_WRITE,
                flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
                fflags: 0,
                data: 0,
                udata: ptr::from_ref(event).cast_mut().cast()
            },
        ];
        unsafe {
            libc::kevent(
                self.kq.as_raw_fd(),
                kevents.as_ptr(),
                2,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            );
        }
    }

    pub(crate) fn del_socket(&self, soc: &Socket) {
        let ident = unsafe { soc.0.as_raw_fd() as libc::uintptr_t };
        let kevents: [libc::kevent; 2] = [
            libc::kevent {
                ident: ident,
                filter: libc::EVFILT_READ,
                flags: libc::EV_DELETE,
                fflags: 0,
                data: 0,
                udata: ptr::null_mut(),
            },
            libc::kevent {
                ident: ident,
                filter: libc::EVFILT_WRITE,
                flags: libc::EV_DELETE,
                fflags: 0,
                data: 0,
                udata: ptr::null_mut(),
            },
        ];
        unsafe {
            libc::kevent(
                self.kq.as_raw_fd(),
                kevents.as_ptr(),
                2,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            );
        }
    }

    pub fn add_signal(&self, sig: Signal, event: &Kevent) {
        let ident = sig.number() as libc::uintptr_t;
        let kevent = libc::kevent {
            ident: ident,
            filter: libc::EVFILT_SIGNAL,
            flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            fflags: 0,
            data: 0,
            udata: ptr::from_ref(event).cast_mut().cast(),
        };
        unsafe {
            libc::kevent(
                self.kq.as_raw_fd(),
                &kevent,
                1,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            );
        }
    }

    pub(crate) fn del_signal(&self, sig: Signal) {
        let ident = sig.number() as libc::uintptr_t;
        let kevent = libc::kevent {
            ident: ident,
            filter: libc::EVFILT_SIGNAL,
            flags: libc::EV_DELETE,
            fflags: 0,
            data: 0,
            udata: ptr::null_mut(),
        };
        unsafe {
            libc::kevent(
                self.kq.as_raw_fd(),
                &kevent,
                1,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            );
        }
    }

    #[cfg(feature = "ktimer")]
    fn reset_timer(&self, timeout: libc::intptr_t) {
        let kevent = libc::kevent {
            ident: 1,
            filter: libc::EVFILT_TIMER,
            flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_ONESHOT,
            fflags: 0,
            data: timeout as libc::intptr_t,
            udata: ptr::from_ref(&self.intr_event).cast_mut().cast()
        };
        unsafe {
            libc::kevent(
                self.kq.as_raw_fd(),
                &kevent, 1,
                ptr::null_mut(),0,
                ptr::null_mut(),
            );
        }
    }

    fn wake_up_alarm(&self, deadline: &Deadline) {
        #[cfg(feature = "ktimer")]
        self.reset_timer(deadline.as_millis() as libc::intptr_t);
        #[cfg(not(feature = "ktimer"))]
        self.intr.wake_up_alarm(deadline);
    }

    fn wake_up_now(&self) {
        #[cfg(feature = "ktimer")]
        self.reset_timer(0);
        #[cfg(not(feature = "ktimer"))]
        self.intr.wake_up_now();
    }

    fn cancel(ev: &Kevent, vec: &mut Vec<Waker>) {
        let mut readable = State::Cancel;
        let mut writable = State::Cancel;
        let mut signaled = State::Cancel;
        let mut ev = ev.inner.lock().unwrap();
        mem::swap(&mut ev.readable, &mut readable);
        mem::swap(&mut ev.writable, &mut writable);
        mem::swap(&mut ev.signaled, &mut signaled);
        drop(ev);
        if let State::Queued(waker) = readable {
            vec.push(waker);
        }
        if let State::Queued(waker) = writable {
            vec.push(waker);
        }
        if let State::Queued(waker) = signaled {
            vec.push(waker);
        }
    }

    pub(in super::super) fn cancel_all_events(&self, scheduler: &Scheduler) {
        let mut vec = Vec::new();
        scheduler.clear_all(&mut vec, Self::cancel);
        for waker in vec {
            waker.wake();
        }
        self.wake_up_now();
    }


    fn kevent<const N: usize>(
        &self,
        events: &mut [MaybeUninit<libc::kevent>; N],
    ) -> Result<usize, OsError> {
        unsafe {
            match libc::kevent(
                unsafe { self.kq.as_raw_fd() },
                ptr::null_mut(), 0,
                events[0].as_mut_ptr(),
                events.len() as i32,
                #[cfg(feature = "ktimer")]
                ptr::null_mut(),
                #[cfg(not(feature = "ktimer"))]
                &mut self.intr.timeout_kqueue(),
            ) {
                -1 => Err(OsError::last()),
                len => Ok(len as usize),
            }
        }
    }

    pub(in super::super) fn poll(&self, scheduler: &Scheduler) -> Poll<OsError> {
        loop {
            const EVENTLEN: usize = 128;
            let mut kevents: [MaybeUninit<libc::kevent>; EVENTLEN] =
                [const { MaybeUninit::uninit() }; EVENTLEN];
            match self.kevent(&mut kevents) {
                Err(OsError::INTERRUPTED) => continue,
                Err(err) => return Poll::Ready(err),
                Ok(len) => {
                    let kevents: [libc::kevent; EVENTLEN] = unsafe { mem::transmute(kevents) };
                    let mut wakers = Vec::new();
                    scheduler.clear_overdue(&mut wakers, Self::cancel);
                    for kev in &kevents[..len] {
                        let event: &Kevent = unsafe { &*kev.udata.cast() };
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event();
                            continue;
                        }
                        if kev.filter == libc::EVFILT_READ {
                            let mut state = State::Ready(0);
                            let mut event = event.inner.lock().unwrap();
                            mem::swap(&mut state, &mut event.readable);
                            drop(event);
                            if let State::Queued(waker) = state {
                                wakers.push(waker);
                            }
                        }
                        if kev.filter == libc::EVFILT_WRITE {
                            let mut state = State::Ready(0);
                            let mut event = event.inner.lock().unwrap();
                            mem::swap(&mut state, &mut event.writable);
                            drop(event);
                            if let State::Queued(waker) = state {
                                wakers.push(waker);
                            }
                        }
                        if (kev.filter == libc::EVFILT_SIGNAL) {
                            let mut state = State::Ready(kev.ident);
                            let mut event = event.inner.lock().unwrap();
                            mem::swap(&mut state, &mut event.signaled);
                            drop(event);
                            if let State::Queued(waker) = state {
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
