use super::{Intr, Scheduler};
use crate::error::OsError;
use crate::primitive::{Deadline, Fd, Timeout};
use std::mem;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

enum State {
    Ready,
    Cancel,
    Queued(Waker),
}

impl State {
    fn ready(&mut self) -> Option<Waker> {
        let mut state = State::Ready;
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
}

impl Inner {
    const fn new() -> Self {
        Self {
            readable: State::Ready,
            writable: State::Ready,
        }
    }
}

pub(crate) struct WaitForReadable<'a, T> {
    event: &'a EpollEvent<T>,
    guard: Option<EpollEventGuard<'a, T>>,
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
                State::Ready => Poll::Ready(Ok(())),
                State::Cancel => Poll::Ready(Err(())),
                _ => Poll::Pending,
            }
        }
    }
}

unsafe impl<'a, T> Send for WaitForReadable<'a, T> {}

unsafe impl<'a, T> Sync for WaitForReadable<'a, T> {}

pub(crate) struct WaitForWritable<'a, T> {
    guard: Option<EpollEventGuard<'a, T>>,
    event: &'a EpollEvent<T>,
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
                State::Ready => Poll::Ready(Ok(())),
                State::Cancel => Poll::Ready(Err(())),
                _ => Poll::Pending,
            }
        }
    }
}

pub(crate) struct EpollEventGuard<'a, T> {
    mutex: MutexGuard<'a, Inner>,
    event: &'a EpollEvent<T>,
}

impl<'a, T> EpollEventGuard<'a, T> {
    pub fn poll_in(self, t: Timeout) -> WaitForReadable<'a, T> {
        WaitForReadable {
            event: self.event,
            guard: Some(self),
        }
    }

    pub fn poll_out(self, t: Timeout) -> WaitForWritable<'a, T> {
        WaitForWritable {
            event: self.event,
            guard: Some(self),
        }
    }
}

#[derive(Clone)]
pub(crate) struct EpollEvent<T>(Arc<(Mutex<Inner>, T)>);

impl<T> EpollEvent<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new((Mutex::new(Inner::new()), data)))
    }

    pub fn as_data(&self) -> &T {
        &self.0.1
    }

    pub fn lock(&self) -> EpollEventGuard<'_, T> {
        EpollEventGuard {
            mutex: self.0.0.lock().unwrap(),
            event: self,
        }
    }

    pub fn cancel(&self, wakers: &mut Vec<Waker>) {
        let mut event = self.0.0.lock().unwrap();
        if let Some(waker) = event.readable.cancel() {
            wakers.push(waker);
        }
        if let Some(waker) = event.writable.cancel() {
            wakers.push(waker);
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

fn epoll_add<T>(epfd: &Fd, fd: &Fd, ev: &EpollEvent<T>, events: u32) {
    let mut event = libc::epoll_event {
        events: events,
        data: libc::epoll_data {
            ptr: Arc::into_raw(ev.0.clone()).cast_mut().cast(),
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

    pub fn add_socket<T>(&self, fd: &Fd, event: &EpollEvent<T>) {
        let flags = libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET;
        epoll_add(&self.epfd, fd, event, flags)
    }

    pub fn del_socket(&self, fd: &Fd) {
        epoll_del(&self.epfd, fd)
    }

    pub fn wake_up_now(&self) {
        self.intr.wake_up_now()
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
                    let events: [libc::epoll_event; EVENTLEN] =
                        unsafe { std::mem::transmute(events) };
                    let mut wakers = Vec::new();
                    let now = Deadline::now();
                    for eev in &events[..len] {
                        let event: EpollEvent<()> =
                            EpollEvent(unsafe { Arc::from_raw(eev.data.ptr.cast()) });
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event();
                            continue;
                        }
                        if (eev.events & (libc::EPOLLIN | libc::EPOLLERR | libc::EPOLLHUP)) != 0 {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.readable.ready() {
                                wakers.push(waker);
                            }
                        }
                        if (eev.events & libc::EPOLLOUT) != 0 {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.writable.ready() {
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
