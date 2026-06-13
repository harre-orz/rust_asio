use super::{Intr, Scheduler};
use crate::error::{OsError, Result};
use crate::primitive::{Deadline, Fd, Socket, Timeout};
use std::mem;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

#[derive(Copy, Clone)]
pub enum EventResult {
    Ready,
    Cancel,
}

enum EpollState {
    Result(EventResult),
    Wait(Waker),
}

impl EpollState {
    fn result(&mut self, res: EventResult) -> Option<Waker> {
        let mut state = EpollState::Result(res);
        mem::swap(self, &mut state);
        if let EpollState::Incomplete(waker) = state {
            Some(waker)
        } else {
            None
        }
    }
}

pub struct Inner {
    readable: EpollState,
    writable: EpollState,
}

pub struct WaitForReadable<'a> {
    event: &'a EpollEvent,
    guard: Option<EpollEventGuard<'a>>,
}

impl<'a> Future for WaitForReadable<'a> {
    type Output = EventResult;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut guard) = self.guard.take() {
            guard.0.readable = EpollState::Incomplete(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            if let EpollState::Result(res) = &event.readable {
                Poll::Ready(res.clone())
            } else {
                Poll::Pending
            }
        }
    }
}

unsafe impl<'a> Send for WaitForReadable<'a> {}

unsafe impl<'a> Sync for WaitForReadable<'a> {}

pub struct WaitForWritable<'a> {
    event: &'a EpollEvent,
    guard: Option<EpollEventGuard<'a>>,
}

unsafe impl<'a> Send for WaitForWritable<'a> {}

unsafe impl<'a> Sync for WaitForWritable<'a> {}

impl<'a> Future for WaitForWritable<'a> {
    type Output = EventResult;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Some(mut guard) = self.guard.take() {
            guard.0.readable = EpollState::Incomplete(ctx.waker().clone());
            Poll::Pending
        } else {
            let event = self.event.0.0.lock().unwrap();
            if let EpollState::Result(res) = &event.readable {
                Poll::Ready(res.clone())
            } else {
                Poll::Pending
            }
        }
    }
}

pub struct EpollEventGuard<'a>(MutexGuard<'a, Inner>);

impl<'a> EpollEventGuard<'a> {
    pub fn poll_in(self, event: &'a EpollEvent, t: Timeout) -> WaitForReadable<'a> {
        WaitForReadable {
            event: event,
            guard: Some(self),
        }
    }

    pub fn poll_out(self, event: &'a EpollEvent, t: Timeout) -> WaitForWritable<'a> {
        WaitForWritable {
            event: event,
            guard: Some(self),
        }
    }
}

#[derive(Clone)]
pub(crate) struct EpollEvent(pub Arc<(Mutex<Inner>, Socket)>);

impl EpollEvent {
    pub fn new(soc: Socket) -> Self {
        Self(Arc::new((
            Mutex::new(Inner {
                readable: EpollState::Result(EventResult::Ready),
                writable: EpollState::Result(EventResult::Ready),
            }),
            soc,
        )))
    }

    pub fn as_socket(&self) -> &Socket {
        &self.0.1
    }

    pub fn lock(&self) -> EpollEventGuard<'_> {
        EpollEventGuard(self.0.0.lock().unwrap())
    }

    pub fn cancel(&self, wakers: &mut Vec<Waker>) {
        let mut event = self.0.0.lock().unwrap();
        if let Some(waker) = event.readable.result(EventResult::Cancel) {
            wakers.push(waker);
        }
        if let Some(waker) = event.writable.result(EventResult::Cancel) {
            wakers.push(waker);
        }
    }
}

fn epoll_create() -> Result<Fd> {
    unsafe {
        match libc::epoll_create1(libc::EPOLL_CLOEXEC) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

fn epoll_add(epfd: &Fd, ev: &EpollEvent, events: u32) {
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
            ev.as_socket().0.as_raw_fd(),
            &mut event,
        );
    }
}

fn epoll_del(epfd: &Fd, ev: &EpollEvent) {
    let mut event = libc::epoll_event {
        events: 0,
        data: libc::epoll_data { u64: 0 },
    };
    unsafe {
        libc::epoll_ctl(
            epfd.as_raw_fd(),
            libc::EPOLL_CTL_DEL,
            ev.as_socket().0.as_raw_fd(),
            &mut event,
        );
    }
}

fn epoll_wait<const N: usize>(
    epfd: &Fd,
    events: &mut [MaybeUninit<libc::epoll_event>; N],
    t: i32,
) -> Result<usize> {
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
    intr_event: EpollEvent,
}

impl Drop for Epoll {
    fn drop(&mut self) {
        epoll_del(&self.epfd, &self.intr_event);
    }
}

impl Epoll {
    pub fn new() -> Result<Self> {
        let epfd = epoll_create()?;
        let (intr, fd) = Intr::new()?;
        let intr_event = EpollEvent::new(Socket(fd));
        epoll_add(&epfd, &intr_event, libc::EPOLLIN);
        Ok(Epoll {
            epfd: epfd,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub fn add_socket(&self, event: &EpollEvent) {
        let flags = libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET;
        epoll_add(&self.epfd, event, flags)
    }

    pub fn del_socket(&self, event: &EpollEvent) {
        epoll_del(&self.epfd, event)
    }

    pub fn wake_up_now(&self) {
        self.intr.wake_up_now(&self.intr_event.as_socket().0)
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
                        let event = EpollEvent(unsafe { Arc::from_raw(eev.data.ptr.cast()) });
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event(&self.intr_event.as_socket().0);
                            continue;
                        }
                        if (eev.events & (libc::EPOLLIN | libc::EPOLLERR | libc::EPOLLHUP)) != 0 {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.readable.result(EventResult::Ready) {
                                wakers.push(waker);
                            }
                        }
                        if (eev.events & libc::EPOLLOUT) != 0 {
                            let mut event = event.0.0.lock().unwrap();
                            if let Some(waker) = event.writable.result(EventResult::Cancel) {
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
