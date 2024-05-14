use crate::error::OsError;
use crate::ffi::{Monotonic, ConnectedSocket, Timeout};
use libc;
use std::cmp;
use std::collections::{BTreeSet, HashSet};
use std::hash;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

fn epoll_create() -> Result<OwnedFd, OsError> {
    match unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) } {
        -1 => Err(unsafe { OsError::last() }),
        fd => Ok(unsafe { OwnedFd::from_raw_fd(fd) }),
    }
}


fn timerfd_create() -> Result<OwnedFd, OsError> {
    match unsafe { libc::timerfd_create(libc::CLOCK_MONOTONIC, libc::TFD_CLOEXEC) } {
        -1 => Err(unsafe { OsError::last() }),
        fd => Ok(unsafe { OwnedFd::from_raw_fd(fd) }),
    }
}

fn epoll_add<F>(epfd: &OwnedFd, fd: &F, events: i32, ptr: &EpollEvent)
where
    F: AsRawFd,
{
    let mut event = libc::epoll_event {
        events: events as u32,
        u64: Arc::as_ptr(&ptr.0) as u64,
    };
    match unsafe { libc::epoll_ctl(epfd.as_raw_fd(), libc::EPOLL_CTL_ADD, fd.as_raw_fd(), &mut event) } {
        -1 => {}
        0 => {}
        _ => unreachable!(),
    }
}

fn epoll_del<F>(epfd: &OwnedFd, fd: &F)
where
    F: AsRawFd,
{
    let mut event = libc::epoll_event {
        events: 0,
        u64: 0,
    };
    match unsafe { libc::epoll_ctl(epfd.as_raw_fd(), libc::EPOLL_CTL_DEL, fd.as_raw_fd(), &mut event) } {
        -1 => {}
        0 => {}
        _ => unreachable!(),
    }
}

fn timerfd_settime(tfd: &OwnedFd, time: Monotonic) {
    use std::ptr;

    let it = libc::itimerspec {
        it_interval: libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        it_value: time.as_timerfd_abstime(),
    };
    match unsafe { libc::timerfd_settime(tfd.as_raw_fd(), libc::TFD_TIMER_ABSTIME, &it, ptr::null_mut()) } {
        -1 => {},
        0 => {},
        _ => unreachable!(),
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
    dispatch: fn(&mut Self, &libc::epoll_event, &OwnedFd) -> Option<Waker>,
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

    fn intr() -> Self {
        Self(Arc::new(Mutex::new(Inner {
            waker: None,
            read_op: Op::Wait,
            write_op: Op::Wait,
            dispatch: |_, _, tfd| {
                let mut buf = MaybeUninit::<[u8; 8]>::uninit();
                unsafe {
                    match libc::read(tfd.as_raw_fd(), buf.as_mut_ptr().cast(), 8) {
                        8 => None,
                        _ => panic!(),
                    }
                }
            },
        })))
    }

    pub fn read_reset(self, epoll: &Epoll, timeout: Timeout) {
        let (timer, waker) = {
            let event = DeadlineEpollEvent(Monotonic::now() + timeout, self);
            let mut data = epoll.data.lock().unwrap();
            data.deadline.insert(event);
            let timer = data.deadline.last().unwrap().0;
            let timer = if timer < data.timer {
                data.timer = timer;
                Some(data.timer)
            } else {
                None
            };
            (timer, data.waker.take())
        };
        if let Some(timer) = timer {
            timerfd_settime(&epoll.tfd, timer);
        }
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
            },
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
        let (timer, waker) = {
            let event = DeadlineEpollEvent(Monotonic::now() + timeout, self);
            let mut data = epoll.data.lock().unwrap();
            data.deadline.insert(event);
            let timer = data.deadline.last().unwrap().0;
            let timer = if timer < data.timer {
                data.timer = timer;
                Some(data.timer)
            } else {
                None
            };
            (timer, data.waker.take())
        };
        if let Some(timer) = timer {
            timerfd_settime(&epoll.tfd,timer);
        }
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
            },
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
    timer: Monotonic,
    deadline: BTreeSet<DeadlineEpollEvent>,
}

pub(super) struct Epoll {
    epfd: OwnedFd,
    tfd: OwnedFd,
    intr: EpollEvent,
    data: Mutex<EpollData>,
}

impl Drop for Epoll {
    fn drop(&mut self) {
        epoll_del(&self.epfd, &self.tfd)
    }
}

impl Epoll {
    pub fn new() -> Result<Self, OsError> {
        let epfd = epoll_create()?;
        let tfd = timerfd_create()?;
        let intr = EpollEvent::intr();
        epoll_add(&epfd, &tfd, libc::EPOLLIN, &intr);
        Ok(Epoll {
            epfd,
            tfd,
            intr,
            data: Mutex::new(EpollData {
                waker: None,
                timer: Monotonic::now(),
                deadline: BTreeSet::new(),
            }),
        })
    }

    pub fn register_socket(&self, soc: &ConnectedSocket) -> EpollEvent {
        let event = EpollEvent::socket();
        epoll_add(&self.epfd, soc, libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET, &event);
        event
    }

    pub fn deregister_socket(&self, soc: &ConnectedSocket) {
        epoll_del(&self.epfd, soc);
    }

    pub fn stop(&self) {
        for event in {
            let mut events = HashSet::new();
            let mut data = self.data.lock().unwrap();
            while let Some(event) = data.deadline.pop_first() {
                events.insert(event.1);
            }
            events
        } {
            if let Some(waker) =event.0.lock().unwrap().cancel() {
                waker.wake()
            }
        }
    }

    pub fn poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        let mut wake_up = false;
        loop {
            const EVENTLEN: usize = 128;
            let mut events = MaybeUninit::<[libc::epoll_event; EVENTLEN]>::uninit();
            match unsafe {
                libc::epoll_wait(
                    self.epfd.as_raw_fd(),
                    events.as_mut_ptr().cast(),
                    EVENTLEN as i32,
                    -1,
                )
            } {
                -1 => return Poll::Ready(Err(unsafe { OsError::last() })),
                len => {
                    let events = unsafe { events.assume_init() };
                    let events = &events[..len as usize];
                    let mut timeout_events = HashSet::new();
                    for event in {
                        let event = DeadlineEpollEvent(Monotonic::now(), self.intr.clone());  // intr is dummy.
                        let mut data = self.data.lock().unwrap();
                        data.deadline.split_off(&event)
                    } {
                        timeout_events.insert(event.1);
                    }
                    for ev in events {
                        let event =
                            EpollEvent(unsafe { Arc::from_raw(ev.u64 as *const Mutex<Inner>) });
                        timeout_events.remove(&event);
                        if let Some(waker) = {
                            let mut event = event.0.lock().unwrap();
                            (event.dispatch)(&mut event, &ev, &self.tfd)
                        } {
                            wake_up = true;
                            waker.wake();
                        }
                    }
                    for event in timeout_events {
                        if let Some(waker) = event.0.lock().unwrap().cancel() {
                            wake_up = true;
                            waker.wake()
                        }
                    }
                    if wake_up {
                        let mut data = self.data.lock().unwrap();
                        data.waker = Some(ctx.waker().clone());
                        return Poll::Pending;
                    }
                }
            }
        }
    }
}

#[test]
fn test_ordering() {
    use std::time::Duration;

    let now = Monotonic::now();
    let mut data: BTreeSet<DeadlineEpollEvent> = BTreeSet::new();

    let ev1 = EpollEvent::socket();
    data.insert(DeadlineEpollEvent(now - Duration::new(10, 0), ev1.clone())); // timeout

    let ev2 = EpollEvent::socket();
    data.insert(DeadlineEpollEvent(now + Duration::new(20, 0), ev2.clone()));

    let ev3 = EpollEvent::socket();
    data.insert(DeadlineEpollEvent(now - Duration::new(40, 0), ev3.clone())); // timeout

    let ev4 = EpollEvent::socket();
    data.insert(DeadlineEpollEvent(now - Duration::new(30, 0), ev4.clone())); // timeout

    let ev5 = EpollEvent::socket();
    data.insert(DeadlineEpollEvent(now + Duration::new(50, 0), ev5.clone()));

    let dummy = EpollEvent::socket();
    let mut exp = data.split_off(&DeadlineEpollEvent(now, dummy));
    if let Some(DeadlineEpollEvent(_, ev)) = exp.pop_last() {
        assert_eq!(ev, ev3);
    }
    if let Some(DeadlineEpollEvent(_, ev)) = exp.pop_last() {
        assert_eq!(ev, ev4);
    }
    if let Some(DeadlineEpollEvent(_, ev)) = exp.pop_last() {
        assert_eq!(ev, ev1);
    }
    assert_eq!(exp.is_empty(), true);
}
