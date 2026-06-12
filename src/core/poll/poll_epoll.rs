use super::{Deadline, EventScheduler, Intr};
use crate::error::{OsError, Result};
use crate::primitive::Fd;
use std::mem;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Debug)]
enum EventOp {
    Ready,
    Pending(Waker),
    Canceled,
    Neutral,
}

#[derive(Debug)]
pub(crate) struct EpollEvent {
    readable_op: EventOp,
    writable_op: EventOp,
}

impl EpollEvent {
    pub(crate) fn read_poll(&mut self, ctx: &mut Context) -> Poll<Result<()>> {
        match self.readable_op {
            EventOp::Ready => {
                self.readable_op = EventOp::Neutral;
                Poll::Ready(Ok(()))
            }
            EventOp::Pending(_) => Poll::Pending,
            EventOp::Canceled => {
                self.readable_op = EventOp::Neutral;
                Poll::Ready(Err(OsError::OPERATION_CANCELED))
            }
            EventOp::Neutral => {
                self.readable_op = EventOp::Pending(ctx.waker().clone());
                Poll::Pending
            }
        }
    }

    pub(crate) fn write_poll(&mut self, ctx: &mut Context) -> Poll<Result<()>> {
        match self.writable_op {
            EventOp::Neutral => {
                self.writable_op = EventOp::Pending(ctx.waker().clone());
                Poll::Pending
            }
            EventOp::Pending(_) => Poll::Pending,
            EventOp::Ready => {
                self.writable_op = EventOp::Neutral;
                Poll::Ready(Ok(()))
            }
            EventOp::Canceled => {
                self.writable_op = EventOp::Neutral;
                Poll::Ready(Err(OsError::OPERATION_CANCELED))
            }
        }
    }

    pub(crate) fn cancel(&mut self, vec: &mut Vec<Waker>) {
        let mut event_op = EventOp::Canceled;
        mem::swap(&mut event_op, &mut self.readable_op);
        if let EventOp::Pending(waker) = event_op {
            vec.push(waker);
        }
        let mut event_op = EventOp::Canceled;
        mem::swap(&mut event_op, &mut self.writable_op);
        if let EventOp::Pending(waker) = event_op {
            vec.push(waker);
        }
    }
}

impl Default for EpollEvent {
    fn default() -> Self {
        Self {
            readable_op: EventOp::Ready,
            writable_op: EventOp::Ready,
        }
    }
}

pub type Event = Arc<Mutex<EpollEvent>>;

fn epoll_create() -> Result<Fd> {
    unsafe {
        match libc::epoll_create1(libc::EPOLL_CLOEXEC) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

fn epoll_add(epfd: &Fd, soc: &Fd, events: u32, ev: &Event) {
    let mut event = libc::epoll_event {
        events: events,
        data: libc::epoll_data {
            ptr: Arc::into_raw(ev.clone()).cast_mut().cast(),
        },
    };
    unsafe {
        libc::epoll_ctl(
            epfd.as_raw_fd(),
            libc::EPOLL_CTL_ADD,
            soc.as_raw_fd(),
            &mut event,
        );
    }
}

fn epoll_del(epfd: &Fd, soc: &Fd) {
    let mut event = libc::epoll_event {
        events: 0,
        data: libc::epoll_data { u64: 0 },
    };
    unsafe {
        libc::epoll_ctl(
            epfd.as_raw_fd(),
            libc::EPOLL_CTL_DEL,
            soc.as_raw_fd(),
            &mut event,
        );
    }
}

fn epoll_wait<const N: usize>(
    epfd: &Fd,
    events: &mut [MaybeUninit<libc::epoll_event>; N],
    timeout: i32,
) -> Result<usize> {
    unsafe {
        match libc::epoll_wait(epfd.as_raw_fd(), events[0].as_mut_ptr(), N as i32, timeout) {
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub struct Epoll {
    epfd: Fd,
    pub(crate) intr: Intr,
    intr_event: Event,
}

impl Drop for Epoll {
    fn drop(&mut self) {
        epoll_del(&self.epfd, self.intr.as_fd())
    }
}

impl Epoll {
    pub(crate) fn new() -> Result<Self> {
        let epfd = epoll_create()?;
        let intr = Intr::new()?;
        let intr_event: Event = Default::default();
        epoll_add(&epfd, intr.as_fd(), libc::EPOLLIN, &intr_event);
        Ok(Epoll {
            epfd: epfd,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn add_socket(&self, soc: &Fd, event: &Event) {
        epoll_add(
            &self.epfd,
            soc,
            libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET,
            event,
        )
    }

    pub(crate) fn del_socket(&self, soc: &Fd) {
        epoll_del(&self.epfd, soc);
    }

    pub(crate) fn poll(&self, scheduler: &EventScheduler) -> Poll<OsError> {
        let mut wakers = Vec::new();
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
                    let now = Deadline::now();
                    for eev in &events[..len] {
                        let event: Event = unsafe { Arc::from_raw(eev.data.ptr.cast()) };
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event();
                            continue;
                        }
                        let mut readable = false;
                        let mut writable = false;
                        if (eev.events & (libc::EPOLLERR | libc::EPOLLHUP)) != 0 {
                            readable = true;
                            writable = true;
                        } else {
                            if (eev.events & libc::EPOLLIN) != 0 {
                                readable = true;
                            }
                            if (eev.events & libc::EPOLLOUT) != 0 {
                                writable = true;
                            }
                        }
                        {
                            let mut event = event.lock().unwrap();
                            if readable {
                                let mut event_op = EventOp::Ready;
                                mem::swap(&mut event_op, &mut event.readable_op);
                                if let EventOp::Pending(waker) = event_op {
                                    wakers.push(waker);
                                }
                            }
                            if writable {
                                let mut event_op = EventOp::Ready;
                                mem::swap(&mut event_op, &mut event.writable_op);
                                if let EventOp::Pending(waker) = event_op {
                                    wakers.push(waker);
                                }
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
