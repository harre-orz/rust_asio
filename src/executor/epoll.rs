use crate::error::OsError;
use crate::ffi::{ConnectedSocket, Timeout};
use libc;
use std::cmp;
use std::collections::{BTreeSet, HashSet};
use std::hash;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Instant;

#[derive(Default, Debug)]
struct Inner {
    waker: Option<Waker>,
    read_op: Option<OsError>,
    write_op: Option<OsError>,
}

#[derive(Clone, Debug)]
pub(super) struct EpollEvent(Arc<Mutex<Inner>>);

struct DeadlineEpollEvent(Instant, EpollEvent);

impl cmp::PartialEq for DeadlineEpollEvent {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl cmp::Eq for DeadlineEpollEvent {}

impl cmp::PartialOrd for DeadlineEpollEvent {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for DeadlineEpollEvent {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.0.cmp(&other.0).reverse() {
            cmp::Ordering::Equal => self.1.cmp(&other.1),
            cmp => cmp,
        }
    }
}

impl EpollEvent {
    pub fn read_reset(self, epoll: &Epoll, timeout: Timeout) {
        if let Some(waker) = {
            let event = DeadlineEpollEvent(timeout.into_expire(), self);
            let mut data = epoll.data.lock().unwrap();
            data.timer.insert(event);
            data.waker.take()
        } {
            waker.wake()
        }
    }

    pub fn read_poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        let mut event = self.0.lock().unwrap();
        match event.read_op.take() {
            None => {
                event.waker = Some(ctx.waker().clone());
                Poll::Pending
            }
            Some(OsError::READY) => Poll::Ready(Ok(())),
            Some(err) => Poll::Ready(Err(err)),
        }
    }

    pub fn write_reset(self, epoll: &Epoll, timeout: Timeout) {
        if let Some(waker) = {
            let event = DeadlineEpollEvent(timeout.into_expire(), self);
            let mut data = epoll.data.lock().unwrap();
            data.timer.insert(event);
            data.waker.take()
        } {
            waker.wake()
        }
    }

    pub fn write_poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        let mut event = self.0.lock().unwrap();
        match event.write_op.take() {
            None => {
                event.waker = Some(ctx.waker().clone());
                Poll::Pending
            }
            Some(OsError::READY) => Poll::Ready(Ok(())),
            Some(err) => Poll::Ready(Err(err)),
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


#[derive(Default)]
struct EpollData {
    waker: Option<Waker>,
    timer: BTreeSet<DeadlineEpollEvent>,
}

pub(super) struct Epoll {
    epfd: OwnedFd,
    data: Mutex<EpollData>,
    sfd: EpollEvent,
}

impl Epoll {
    pub fn new() -> Result<Self, OsError> {
        match unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) } {
            -1 => Err(unsafe { OsError::last() }),
            epfd => {
                let epfd = unsafe { OwnedFd::from_raw_fd(epfd) };
                Ok(Epoll {
                    epfd,
                    data: Default::default(),
                    sfd: EpollEvent(Default::default()),
                })
            }
        }
    }

    fn epoll_ctl(&self, soc: &ConnectedSocket, op: i32, ptr: u64) {
        let mut event = libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET) as u32,
            u64: ptr,
        };
        match unsafe { libc::epoll_ctl(self.epfd.as_raw_fd(), op, soc.as_raw_fd(), &mut event) } {
            -1 => {}
            0 => {}
            _ => unreachable!(),
        }
    }

    pub fn register_socket(&self, soc: &ConnectedSocket) -> EpollEvent {
        let event = Default::default();
        self.epoll_ctl(soc, libc::EPOLL_CTL_ADD, Arc::as_ptr(&event) as u64);
        EpollEvent(event)
    }

    pub fn deregister_socket(&self, soc: &ConnectedSocket) {
        self.epoll_ctl(soc, libc::EPOLL_CTL_DEL, 0);
    }

    pub fn stop(&self) {
        for event in {
            let mut events = HashSet::new();
            let mut data = self.data.lock().unwrap();
            while let Some(event) = data.timer.pop_first() {
                events.insert(event.1);
            }
            events
        } {
            if let Some(waker) = {
                let mut event = event.0.lock().unwrap();
                event.read_op = Some(OsError::OPERATION_CANCELED);
                event.write_op = Some(OsError::OPERATION_CANCELED);
                event.waker.take()
            } {
                waker.wake()
            }
        }
    }

    pub fn poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        let mut wake_up = false;
        loop {
            const EVENTLEN: usize = 128;
            let mut events = MaybeUninit::<[libc::epoll_event; EVENTLEN]>::uninit();
            let timeout = {
                let now = Instant::now();
                let data = self.data.lock().unwrap();
                if let Some(kv) = data.timer.first() {
                    let millis = (kv.0 - now).as_millis();
                    if millis > i32::MAX as u128 {
                        -1
                    } else {
                        millis as i32
                    }
                } else {
                    return Poll::Ready(Ok(()));
                }
            };
            match unsafe {
                libc::epoll_wait(
                    self.epfd.as_raw_fd(),
                    events.as_mut_ptr().cast(),
                    EVENTLEN as i32,
                    timeout,
                )
            } {
                -1 => return Poll::Ready(Err(unsafe { OsError::last() })),
                len => {
                    let events = unsafe { events.assume_init() };
                    let events = &events[..len as usize];
                    let mut timeout_events = HashSet::new();
                    for event in {
                        let event = DeadlineEpollEvent(Instant::now(), self.sfd.clone());// sfd is dummy.
                        let mut data = self.data.lock().unwrap();
                        data.timer.split_off(&event)
                    } {
                        timeout_events.insert(event.1);
                    }
                    for ev in events {
                        let event =
                            EpollEvent(unsafe { Arc::from_raw(ev.u64 as *const Mutex<Inner>) });
                        timeout_events.remove(&event);
                        if let Some(waker) = {
                            let mut event = event.0.lock().unwrap();
                            if (ev.events & (libc::EPOLLERR | libc::EPOLLHUP) as u32) != 0 {
                                event.read_op = Some(OsError::CONNECTION_ABORTED);
                                event.write_op = Some(OsError::CONNECTION_ABORTED);
                            } else {
                                if (ev.events & libc::EPOLLIN as u32) != 0 {
                                    event.read_op = Some(OsError::READY);
                                }
                                if (ev.events & libc::EPOLLOUT as u32) != 0 {
                                    event.write_op = Some(OsError::READY);
                                }
                            }
                            event.waker.take()
                        } {
                            wake_up = true;
                            waker.wake();
                        }
                    }
                    for event in timeout_events {
                        if let Some(waker) = {
                            let mut event = event.0.lock().unwrap();
                            event.read_op = Some(OsError::OPERATION_CANCELED);
                            event.write_op = Some(OsError::OPERATION_CANCELED);
                            event.waker.take()
                        } {
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

    let now = Instant::now();
    let mut data: BTreeSet<DeadlineEpollEvent> = BTreeSet::new();

    let ev1 = EpollEvent(Default::default());
    data.insert(DeadlineEpollEvent(now - Duration::new(10, 0), ev1.clone())); // timeout

    let ev2 = EpollEvent(Default::default());
    data.insert(DeadlineEpollEvent(now + Duration::new(10, 0), ev2.clone()));

    let ev3 = EpollEvent(Default::default());
    data.insert(DeadlineEpollEvent(now - Duration::new(10, 0), ev3.clone())); // timeout

    let ev4 = EpollEvent(Default::default());
    data.insert(DeadlineEpollEvent(now - Duration::new(10, 0), ev4.clone())); // timeout

    let ev5 = EpollEvent(Default::default());
    data.insert(DeadlineEpollEvent(now + Duration::new(10, 0), ev5.clone()));

    let dummy = EpollEvent(Default::default());
    let mut exp = data.split_off(&DeadlineEpollEvent(now, dummy));
    if let Some(DeadlineEpollEvent(_, ev)) = exp.pop_last() {
        assert_eq!(ev, ev4);
    }
    if let Some(DeadlineEpollEvent(_, ev)) = exp.pop_last() {
        assert_eq!(ev, ev3);
    }
    if let Some(DeadlineEpollEvent(_, ev)) = exp.pop_last() {
        assert_eq!(ev, ev1);
    }
    assert_eq!(exp.is_empty(), true);
}
