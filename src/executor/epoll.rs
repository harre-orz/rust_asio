use super::{DeadlineEventSet, Intr};
use crate::error::OsError;
use crate::ffi::{Monotonic, Socket, Timeout};
use libc;
use std::cmp;
use std::collections::HashSet;
use std::hash;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

mod ffi {
    use crate::executor::epoll::EpollEvent;
    use crate::executor::epoll::OsError;
    use libc;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::sync::Arc;

    pub fn epoll_create() -> Result<OwnedFd, OsError> {
        match unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) } {
            -1 => Err(unsafe { OsError::last() }),
            fd => Ok(unsafe { OwnedFd::from_raw_fd(fd) }),
        }
    }

    pub fn epoll_add<F>(epfd: &OwnedFd, fd: &F, events: i32, event: &EpollEvent)
    where
        F: AsRawFd,
    {
        let mut event = libc::epoll_event {
            events: events as u32,
            u64: Arc::as_ptr(&event.0) as u64,
        };
        match unsafe {
            libc::epoll_ctl(
                epfd.as_raw_fd(),
                libc::EPOLL_CTL_ADD,
                fd.as_raw_fd(),
                &mut event,
            )
        } {
            -1 => panic!(),
            0 => {}
            _ => unreachable!(),
        }
    }

    pub fn epoll_del<F>(epfd: &OwnedFd, fd: &F)
    where
        F: AsRawFd,
    {
        let mut event = libc::epoll_event { events: 0, u64: 0 };
        match unsafe {
            libc::epoll_ctl(
                epfd.as_raw_fd(),
                libc::EPOLL_CTL_DEL,
                fd.as_raw_fd(),
                &mut event,
            )
        } {
            -1 => panic!(),
            0 => {}
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
enum Op {
    Wait,
    Ready,
    Cancel,
}

#[derive(Debug)]
struct Inner {
    waker: Option<Waker>,
    read_op: Op,
    write_op: Op,
    dispatch: fn(&mut Self, &libc::epoll_event, &Intr) -> Option<Waker>,
}

impl Inner {
    fn cancel(&mut self) -> Option<Waker> {
        self.read_op = Op::Cancel;
        self.write_op = Op::Cancel;
        self.waker.take()
    }
}

#[derive(Clone, Debug)]
pub(super) struct EpollEvent(Arc<Mutex<Inner>>);

impl EpollEvent {
    fn socket() -> Self {
        Self(Arc::new(Mutex::new(Inner {
            waker: None,
            read_op: Op::Wait,
            write_op: Op::Wait,
            dispatch: |event, ev, _| {
                if (ev.events & (libc::EPOLLERR | libc::EPOLLHUP) as u32) != 0 {
                    event.read_op = Op::Cancel;
                    event.write_op = Op::Cancel;
                } else {
                    if (ev.events & libc::EPOLLIN as u32) != 0 {
                        event.read_op = Op::Ready;
                    }
                    if (ev.events & libc::EPOLLOUT as u32) != 0 {
                        event.write_op = Op::Ready;
                    }
                }
                event.waker.take()
            },
        })))
    }

    pub fn intr() -> Self {
        Self(Arc::new(Mutex::new(Inner {
            waker: None,
            read_op: Op::Wait,
            write_op: Op::Wait,
            dispatch: |_, _, intr| {
                intr.read();
                None
            },
        })))
    }

    pub fn read_reset(self, epoll: &Epoll, timeout: Timeout) {
        let waker = {
            let deadline = Monotonic::now() + timeout;
            let mut data = epoll.data.lock().unwrap();
            if let Some(deadline) = data.timer.insert_event(self, deadline) {
                epoll.intr.reset(deadline)
            }
            data.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake()
        }
    }

    pub fn read_poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        let mut event = self.0.lock().unwrap();
        match event.read_op {
            Op::Wait => {
                event.waker = Some(ctx.waker().clone());
                Poll::Pending
            }
            Op::Ready => {
                event.read_op = Op::Wait;
                Poll::Ready(Ok(()))
            }
            Op::Cancel => {
                event.read_op = Op::Wait;
                event.write_op = Op::Wait;
                Poll::Ready(Err(OsError::OPERATION_CANCELED))
            }
        }
    }

    pub fn write_reset(self, epoll: &Epoll, timeout: Timeout) {
        let waker = {
            let deadline = Monotonic::now() + timeout;
            let mut data = epoll.data.lock().unwrap();
            if let Some(deadline) = data.timer.insert_event(self, deadline) {
                epoll.intr.reset(deadline)
            }
            data.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake()
        }
    }

    pub fn write_poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        let mut event = self.0.lock().unwrap();
        match event.write_op {
            Op::Wait => {
                event.waker = Some(ctx.waker().clone());
                Poll::Pending
            }
            Op::Ready => {
                event.write_op = Op::Wait;
                Poll::Ready(Ok(()))
            }
            Op::Cancel => {
                event.read_op = Op::Wait;
                event.write_op = Op::Wait;
                Poll::Ready(Err(OsError::OPERATION_CANCELED))
            }
        }
    }
}

impl cmp::PartialEq for EpollEvent {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.0) == Arc::as_ptr(&other.0)
    }
}

impl cmp::Eq for EpollEvent {}

impl cmp::PartialOrd for EpollEvent {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for EpollEvent {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        Arc::as_ptr(&self.0).cmp(&Arc::as_ptr(&other.0))
    }
}

impl hash::Hash for EpollEvent {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        hasher.write_usize(Arc::as_ptr(&self.0) as usize)
    }
}

#[derive(PartialOrd, PartialEq, Eq)]
struct DeadlineEpollEvent(Monotonic, EpollEvent);

impl cmp::Ord for DeadlineEpollEvent {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.0.cmp(&other.0).reverse() {
            cmp::Ordering::Equal => self.1.cmp(&other.1),
            cmp => cmp,
        }
    }
}

struct EpollData {
    waker: Option<Waker>,
    timer: DeadlineEventSet,
}

pub(super) struct Epoll {
    epfd: OwnedFd,
    intr: Intr,
    data: Mutex<EpollData>,
}

impl Drop for Epoll {
    fn drop(&mut self) {
        ffi::epoll_del(&self.epfd, &self.intr)
    }
}

impl Epoll {
    pub fn new() -> Result<Self, OsError> {
        let epfd = ffi::epoll_create()?;
        let intr = Intr::new()?;
        ffi::epoll_add(&epfd, &intr, libc::EPOLLIN, intr.as_event());
        Ok(Epoll {
            epfd,
            intr,
            data: Mutex::new(EpollData {
                waker: None,
                timer: DeadlineEventSet::new(),
            }),
        })
    }

    pub fn register_socket(&self, soc: &Socket) -> EpollEvent {
        let event = EpollEvent::socket();
        ffi::epoll_add(
            &self.epfd,
            soc,
            libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET,
            &event,
        );
        event
    }

    pub fn deregister_socket(&self, soc: &Socket) {
        ffi::epoll_del(&self.epfd, soc);
    }

    pub fn stop_request(&self) {
        self.intr.intr()
    }

    pub fn stop(&self) {
        for event in {
            let mut data = self.data.lock().unwrap();
            data.timer.into_hashset()
        } {
            if let Some(waker) = event.0.lock().unwrap().cancel() {
                waker.wake()
            }
        }
    }

    pub fn poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        let mut wake_up = false;
        loop {
            let timeout = self.intr.timeout_for_epoll();
            const EVENTLEN: usize = 128;
            let mut events = MaybeUninit::<[libc::epoll_event; EVENTLEN]>::uninit();
            match unsafe {
                libc::epoll_wait(
                    self.epfd.as_raw_fd(),
                    events.as_mut_ptr().cast(),
                    EVENTLEN as i32,
                    timeout,
                )
            } {
                -1 => match unsafe { OsError::last() } {
                    OsError::INTERRUPTED => {}
                    err => return Poll::Ready(Err(err)),
                },
                len => {
                    let events = unsafe { events.assume_init() };
                    let events = &events[..len as usize];
                    let mut rw_events = HashSet::new();
                    for ev in events {
                        let event =
                            EpollEvent(unsafe { Arc::from_raw(ev.u64 as *const Mutex<Inner>) });
                        if let Some(waker) = {
                            let mut event = event.0.lock().unwrap();
                            (event.dispatch)(&mut event, ev, &self.intr)
                        } {
                            wake_up = true;
                            waker.wake()
                        }
                        rw_events.insert(event);
                    }
                    let now = Monotonic::now();
                    for event in {
                        let mut data = self.data.lock().unwrap();
                        if data.timer.is_empty() {
                            return Poll::Ready(Ok(()));
                        }
                        data.timer.remove_events(&rw_events);
                        data.timer
                            .detach_timeout_events(self.intr.as_event().clone(), now)
                    } {
                        if let Some(waker) = event.0 .0.lock().unwrap().cancel() {
                            wake_up = true;
                            waker.wake()
                        }
                    }
                    if wake_up {
                        let mut data = self.data.lock().unwrap();
                        if let Some(deadline) = data.timer.update_timeout() {
                            self.intr.reset(deadline)
                        }
                        data.waker = Some(ctx.waker().clone());
                        return Poll::Pending;
                    }
                }
            }
        }
    }
}
