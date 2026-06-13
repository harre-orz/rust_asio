use super::{Intr, Scheduler};
use crate::error::{OsError, Result};
use crate::primitive::{Deadline, Fd, Socket};
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

struct Op {
    readable_op: EventOp,
    writable_op: EventOp,
}

impl Op {
    fn read_poll(&mut self, ctx: &mut Context) -> Poll<Result<()>> {
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

    fn write_poll(&mut self, ctx: &mut Context) -> Poll<Result<()>> {
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

    fn cancel(&mut self, vec: &mut Vec<Waker>) {
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

#[derive(Clone)]
pub(crate) struct EpollEvent(Arc<(Socket, Mutex<Op>)>);

impl EpollEvent {
    pub fn new(soc: Socket) -> Self {
        Self(Arc::new((
            soc,
            Mutex::new(Op {
                readable_op: EventOp::Ready,
                writable_op: EventOp::Ready,
            }),
        )))
    }

    pub fn as_socket(&self) -> &Socket {
        &self.0.0
    }

    pub fn read_poll(&self, ctx: &mut Context) -> Poll<Result<()>> {
        Poll::Pending
    }

    pub fn write_poll(&self, ctx: &mut Context) -> Poll<Result<()>> {
        Poll::Pending
    }

    pub fn cancel(&self, vec: &mut Vec<Waker>) {
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
    timeout: i32,
) -> Result<usize> {
    unsafe {
        match libc::epoll_wait(epfd.as_raw_fd(), events[0].as_mut_ptr(), N as i32, timeout) {
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
        epoll_add(
            &self.epfd,
            event,
            flags,
        )
    }

    pub fn del_socket(&self, event: &EpollEvent) {
        epoll_del(&self.epfd, event)
    }

    pub fn wake_up_now(&self) {
        self.intr.wake_up_now(&self.intr_event.as_socket().0)
    }

    pub fn poll(&self, scheduler: &Scheduler) -> Poll<OsError> {
        let mut wakers = Vec::new();
        loop {
            const EVENTLEN: usize = 128;
            let mut events: [MaybeUninit<libc::epoll_event>; EVENTLEN] =
                [const { MaybeUninit::uninit() }; EVENTLEN];
            match epoll_wait(
                &self.epfd,
                &mut events,
                self.intr.timeout_epoll(),
            ) {
                Err(OsError::INTERRUPTED) => continue,
                Err(err) => return Poll::Ready(err),
                Ok(len) => {
                    let events: [libc::epoll_event; EVENTLEN] =
                        unsafe { std::mem::transmute(events) };
                    let now = Deadline::now();
                    for eev in &events[..len] {
                        let event = EpollEvent(unsafe { Arc::from_raw(eev.data.ptr.cast()) });
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event(&self.intr_event.as_socket().0);
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
                            let mut event = event.0.1.lock().unwrap();
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
