use super::{Deadline, Intr, IoContext, Scheduler};
use crate::error::OsError;
use crate::primitive::{Fd, Socket, Timeout};
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

impl Inner {
    const fn new() -> Self {
        Self {
            readable: State::Ready,
            writable: State::Ready,
        }
    }
}

pub(crate) struct WaitForReadable<'a, 'b> {
    mutex: Option<MutexGuard<'a, Inner>>,
    timer: Timeout,
    event: &'a Mutex<Inner>,
    inner: &'b super::super::Inner,
}

impl<'a, 'b> Future for WaitForReadable<'a, 'b> {
    type Output = Result<(), ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut mutex) = self.mutex.take() {
            mutex.readable = State::Queued(ctx.waker().clone());
            drop(mutex);
            if let Some(deadline) = self
                .inner
                .scheduler
                .add_event(ptr::from_ref(self.event).cast(), self.timer)
            {
                self.inner.reactor.wake_up_alarm(deadline)
            }
            Poll::Pending
        } else {
            let event = unsafe { &*self.event }.lock().unwrap();
            match &event.readable {
                State::Ready => {
                    drop(event);
                    if let Some(deadline) = self
                        .inner
                        .scheduler
                        .del_event(ptr::from_ref(self.event).cast())
                    {
                        self.inner.reactor.wake_up_alarm(deadline)
                    }
                    Poll::Ready(Ok(()))
                }
                State::Cancel => {
                    drop(event);
                    if let Some(deadline) = self
                        .inner
                        .scheduler
                        .del_event(ptr::from_ref(self.event).cast())
                    {
                        self.inner.reactor.wake_up_alarm(deadline)
                    }
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
    mutex: Option<MutexGuard<'a, Inner>>,
    timer: Timeout,
    event: &'a Mutex<Inner>,
    inner: &'b super::super::Inner,
}

unsafe impl<'a, 'b> Send for WaitForWritable<'a, 'b> {}

unsafe impl<'a, 'b> Sync for WaitForWritable<'a, 'b> {}

impl<'a, 'b> Future for WaitForWritable<'a, 'b> {
    type Output = Result<(), ()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut mutex) = self.mutex.take() {
            mutex.writable = State::Queued(ctx.waker().clone());
            drop(mutex);
            if let Some(deadline) = self
                .inner
                .scheduler
                .add_event(ptr::from_ref(self.event).cast(), self.timer)
            {
                self.inner.reactor.wake_up_alarm(deadline)
            }
            Poll::Pending
        } else {
            let event = unsafe { &*self.event }.lock().unwrap();
            match &event.writable {
                State::Ready => {
                    drop(event);
                    if let Some(deadline) = self
                        .inner
                        .scheduler
                        .del_event(ptr::from_ref(self.event).cast())
                    {
                        self.inner.reactor.wake_up_alarm(deadline)
                    }
                    Poll::Ready(Ok(()))
                }
                State::Cancel => {
                    drop(event);
                    if let Some(deadline) = self
                        .inner
                        .scheduler
                        .del_event(ptr::from_ref(self.event).cast())
                    {
                        self.inner.reactor.wake_up_alarm(deadline)
                    }
                    Poll::Ready(Err(()))
                }
                State::Queued(_) => Poll::Pending,
            }
        }
    }
}

pub(crate) struct EpollEventGuard<'a, 'b> {
    mutex: MutexGuard<'a, Inner>,
    event: &'a Mutex<Inner>,
    inner: &'b super::super::Inner,
}

impl<'a, 'b> EpollEventGuard<'a, 'b> {
    pub fn poll_in(self, t: Timeout) -> WaitForReadable<'a, 'b> {
        WaitForReadable {
            timer: t,
            event: self.event,
            mutex: Some(self.mutex),
            inner: self.inner,
        }
    }

    pub fn poll_out(self, t: Timeout) -> WaitForWritable<'a, 'b> {
        WaitForWritable {
            timer: t,
            mutex: Some(self.mutex),
            event: self.event,
            inner: self.inner,
        }
    }
}

pub(crate) struct EpollEvent<T>(Box<(Mutex<Inner>, T)>);

impl<T> EpollEvent<T> {
    pub fn new(data: T) -> Self {
        Self(Box::new((Mutex::new(Inner::new()), data)))
    }

    pub fn as_data(&self) -> &T {
        &self.0.1
    }

    pub fn lock<'a, 'b>(&'a self, ctx: &'b IoContext) -> EpollEventGuard<'a, 'b> {
        let event = &self.0.0;
        EpollEventGuard {
            mutex: self.0.0.lock().unwrap(),
            event: event,
            inner: &ctx.inner,
        }
    }

    pub fn cancel(&self, wakers: &mut Vec<Waker>) {
        let mut readable = State::Cancel;
        let mut writable = State::Cancel;
        let mut event = self.0.0.lock().unwrap();
        mem::swap(&mut event.readable, &mut readable);
        mem::swap(&mut event.writable, &mut writable);
        drop(event);
        if let State::Queued(waker) = readable {
            wakers.push(waker);
        }
        if let State::Queued(waker) = writable {
            wakers.push(waker);
        }
    }

    fn cast(&self) -> &EpollEvent<()> {
        unsafe { mem::transmute(self) }
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

fn epoll_add<T>(epfd: &Fd, fd: &Fd, ev: &EpollEvent<T>, events: u32) {
    let mut event = libc::epoll_event {
        events: events,
        data: libc::epoll_data {
            ptr: ptr::from_ref(&*ev.0).cast_mut().cast(),
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

pub(in super::super) struct Epoll {
    epfd: Fd,
    intr: Intr,
    intr_event: EpollEvent<()>,
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
        epoll_add(&epfd, intr.as_fd(), &intr_event, libc::EPOLLIN);
        Ok(Epoll {
            epfd: epfd,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub fn add_socket<T>(&self, soc: &Socket, event: &EpollEvent<T>) {
        let flags = libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET;
        epoll_add(&self.epfd, &soc.0, event, flags)
    }

    pub fn del_socket(&self, soc: &Socket) {
        epoll_del(&self.epfd, &soc.0)
    }

    pub fn wake_up_now(&self) {
        self.intr.wake_up_now()
    }

    fn wake_up_alarm(&self, deadline: Deadline) {
        self.intr.wake_up_alarm(deadline)
    }

    pub fn cancel_all_events(&self, scheduler: &Scheduler) {
        let mut vec = Vec::new();
        scheduler.cancel_all_events(|eev| {
            let mut readable = State::Cancel;
            let mut writable = State::Cancel;
            let mut ev = unsafe { &*(eev as *const Mutex<Inner>) }.lock().unwrap();
            mem::swap(&mut ev.readable, &mut readable);
            mem::swap(&mut ev.writable, &mut writable);
            drop(ev);
            if let State::Queued(waker) = readable {
                vec.push(waker);
            }
            if let State::Queued(waker) = writable {
                vec.push(waker);
            }
        });
        for waker in vec {
            waker.wake();
        }
    }

    pub fn poll(&self, scheduler: &Scheduler) -> Poll<OsError> {
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
                    scheduler.clear_overdue(|ev| {
                        let mut readable = State::Cancel;
                        let mut writable = State::Cancel;
                        let mut event = unsafe { &*(ev as *const Mutex<Inner>) }.lock().unwrap();
                        mem::swap(&mut event.readable, &mut readable);
                        mem::swap(&mut event.writable, &mut writable);
                        drop(event);
                        if let State::Queued(waker) = readable {
                            wakers.push(waker);
                        }
                        if let State::Queued(waker) = writable {
                            wakers.push(waker);
                        }
                    });
                    for eev in &events[..len] {
                        let event: &Mutex<Inner> = unsafe { &*eev.data.ptr.cast() };
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event();
                            continue;
                        }
                        if (eev.events & (libc::EPOLLIN | libc::EPOLLERR | libc::EPOLLHUP)) != 0 {
                            let mut readable = State::Ready;
                            let mut event = event.lock().unwrap();
                            mem::swap(&mut event.readable, &mut readable);
                            drop(event);
                            if let State::Queued(waker) = readable {
                                wakers.push(waker);
                            }
                        }
                        if (eev.events & libc::EPOLLOUT) != 0 {
                            let mut writable = State::Ready;
                            let mut event = event.lock().unwrap();
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
