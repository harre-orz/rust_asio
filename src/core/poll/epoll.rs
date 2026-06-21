use super::{Deadline, Intr, Scheduler};
use crate::error::OsError;
use crate::primitive::{Fd, Socket, Timeout};
use std::cmp::Ordering;
use std::mem;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr;
use std::sync::{Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

enum State {
    Ready,
    Cancel,
    Queued(Waker),
}

struct Inner {
    readable: State,
    writable: State,
}

pub(crate) struct EpollEvent {
    inner: Mutex<Inner>,
    pub(in super::super) deadline: Deadline,
}

impl EpollEvent {
    pub(crate) fn new<T>(data: T) -> Pin<Box<(Self, T)>> {
        Box::pin((
            Self {
                inner: Mutex::new(Inner {
                    readable: State::Ready,
                    writable: State::Ready,
                }),
                deadline: Deadline::UNINIT,
            },
            data,
        ))
    }
}

impl PartialEq for EpollEvent {
    fn eq(&self, other: &Self) -> bool {
        ptr::addr_eq(self, other)
    }
}

impl Eq for EpollEvent {}

impl PartialOrd for EpollEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EpollEvent {
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

pub(crate) struct WaitForReadable<'a, 'b> {
    inner: &'a super::super::Inner,
    event: &'b EpollEvent,
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
                State::Ready => {
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

pub(crate) struct WaitForWritable<'a, 'b> {
    inner: &'a super::super::Inner,
    event: &'b EpollEvent,
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
            match &event.writable {
                State::Ready => {
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

pub(crate) struct EpollEventGuard<'a, 'b> {
    inner: &'a super::super::Inner,
    event: &'b EpollEvent,
    mutex: MutexGuard<'b, Inner>,
}

impl<'a, 'b> EpollEventGuard<'a, 'b> {
    pub(in super::super) fn lock(inner: &'a super::super::Inner, event: &'b EpollEvent) -> Self {
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
}

fn epoll_create() -> Result<Fd, OsError> {
    unsafe {
        match libc::epoll_create1(libc::EPOLL_CLOEXEC) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

fn epoll_add(epfd: &Fd, fd: &Fd, ev: &EpollEvent, events: u32) {
    let mut event = libc::epoll_event {
        events: events,
        data: libc::epoll_data {
            ptr: ptr::from_ref(ev).cast_mut().cast(),
        },
    };
    unsafe {
        libc::epoll_ctl(
            epfd.as_raw_fd(),
            libc::EPOLL_CTL_ADD,
            fd.as_raw_fd(),
            &mut event,
        );
    }
}

fn epoll_del(epfd: &Fd, fd: &Fd) {
    let mut event = libc::epoll_event {
        events: 0,
        data: libc::epoll_data { u64: 0 },
    };
    unsafe {
        libc::epoll_ctl(
            epfd.as_raw_fd(),
            libc::EPOLL_CTL_DEL,
            fd.as_raw_fd(),
            &mut event,
        );
    }
}

fn epoll_wait<const N: usize>(
    epfd: &Fd,
    events: &mut [MaybeUninit<libc::epoll_event>; N],
    t: i32,
) -> Result<usize, OsError> {
    unsafe {
        match libc::epoll_wait(epfd.as_raw_fd(), events[0].as_mut_ptr(), N as i32, t) {
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub(crate) struct Epoll {
    epfd: Fd,
    intr: Intr,
    intr_event: Pin<Box<(EpollEvent, ())>>,
}

impl Drop for Epoll {
    fn drop(&mut self) {
        epoll_del(&self.epfd, self.intr.as_fd());
    }
}

impl Epoll {
    pub fn new() -> Result<Self, OsError> {
        let epfd = epoll_create()?;
        let intr = Intr::new()?;
        let intr_event = EpollEvent::new(());
        epoll_add(&epfd, intr.as_fd(), &intr_event.0, libc::EPOLLIN);
        Ok(Epoll {
            epfd: epfd,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub fn add_socket(&self, soc: &Socket, event: &EpollEvent) {
        let flags = libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET;
        epoll_add(&self.epfd, &soc.0, event, flags)
    }

    pub fn del_socket(&self, soc: &Socket) {
        epoll_del(&self.epfd, &soc.0)
    }

    fn wake_up_now(&self) {
        self.intr.wake_up_now()
    }

    fn wake_up_alarm(&self, deadline: &Deadline) {
        self.intr.wake_up_alarm(deadline)
    }

    fn cancel(ev: &EpollEvent, vec: &mut Vec<Waker>) {
        let mut readable = State::Cancel;
        let mut writable = State::Cancel;
        let mut ev = ev.inner.lock().unwrap();
        mem::swap(&mut ev.readable, &mut readable);
        mem::swap(&mut ev.writable, &mut writable);
        drop(ev);
        if let State::Queued(waker) = readable {
            vec.push(waker);
        }
        if let State::Queued(waker) = writable {
            vec.push(waker);
        }
    }

    pub(in super::super) fn cancel_all_events(&self, scheduler: &Scheduler) {
        let mut vec = Vec::new();
        scheduler.clear_all(&mut vec, Self::cancel);
        for waker in vec {
            waker.wake();
        }
        self.wake_up_now()
    }

    pub(in super::super) fn poll(&self, scheduler: &Scheduler) -> Poll<OsError> {
        loop {
            const EVENTLEN: usize = 128;
            let mut events: [MaybeUninit<libc::epoll_event>; EVENTLEN] =
                [const { MaybeUninit::uninit() }; EVENTLEN];
            match epoll_wait(&self.epfd, &mut events, self.intr.timeout_epoll()) {
                Err(OsError::INTERRUPTED) => continue,
                Err(err) => return Poll::Ready(err),
                Ok(len) => {
                    let events: [libc::epoll_event; EVENTLEN] = unsafe { mem::transmute(events) };
                    let mut wakers = Vec::new();
                    scheduler.clear_overdue(&mut wakers, Self::cancel);
                    for eev in &events[..len] {
                        let event: &EpollEvent = unsafe { &*eev.data.ptr.cast() };
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event();
                            continue;
                        }
                        if (eev.events & (libc::EPOLLIN | libc::EPOLLERR | libc::EPOLLHUP)) != 0 {
                            let mut readable = State::Ready;
                            let mut event = event.inner.lock().unwrap();
                            mem::swap(&mut event.readable, &mut readable);
                            drop(event);
                            if let State::Queued(waker) = readable {
                                wakers.push(waker);
                            }
                        }
                        if (eev.events & libc::EPOLLOUT) != 0 {
                            let mut writable = State::Ready;
                            let mut event = event.inner.lock().unwrap();
                            mem::swap(&mut event.writable, &mut writable);
                            drop(event);
                            if let State::Queued(waker) = writable {
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
