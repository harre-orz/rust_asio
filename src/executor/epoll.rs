use super::{AsyncSocket, IoContext};
use crate::error::OsError;
use crate::ffi::{ConnectedSocket, Timeout};
use libc;
use std::cmp;
use std::collections::BTreeMap;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Instant;

pub struct EpollEvent {
    waker: Option<Waker>,
    read_op: Option<OsError>,
    write_op: Option<OsError>,
}

impl EpollEvent {
    pub fn new() -> Self {
        Self {
            waker: None,
            read_op: None,
            write_op: None,
        }
    }

    pub fn read_reset(
        event: &Arc<Mutex<Self>>,
        soc: &ConnectedSocket,
        timeout: Timeout,
        epoll: &Epoll,
    ) {
        let expire = timeout.into_expire();
        if let Some(waker) = {
            let mut op = event.lock().unwrap();
            op.read_op = None;
            op.waker.take()
        } {
            waker.wake();
        }
        if let Some(waker) = {
            let mut data = epoll.data.lock().unwrap();
            data.timer
                .insert(Key(expire, soc.as_raw_fd()), event.clone());
            data.waker.take()
        } {
            waker.wake()
        }
    }

    pub fn read_poll(&mut self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        match self.read_op.take() {
            None => {
                self.waker = Some(ctx.waker().clone());
                Poll::Pending
            }
            Some(OsError::READY) => Poll::Ready(Ok(())),
            Some(err) => Poll::Ready(Err(err)),
        }
    }

    pub fn write_reset(
        event: &Arc<Mutex<Self>>,
        soc: &ConnectedSocket,
        timeout: Timeout,
        epoll: &Epoll,
    ) {
        let expire = timeout.into_expire();
        if let Some(waker) = {
            let mut op = event.lock().unwrap();
            op.write_op = None;
            op.waker.take()
        } {
            waker.wake();
        }
        if let Some(waker) = {
            let mut data = epoll.data.lock().unwrap();
            data.timer
                .insert(Key(expire, soc.as_raw_fd()), event.clone());
            data.waker.take()
        } {
            waker.wake()
        }
    }

    pub fn write_poll(&mut self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        match self.write_op.take() {
            None => {
                self.waker = Some(ctx.waker().clone());
                Poll::Pending
            }
            Some(OsError::READY) => Poll::Ready(Ok(())),
            Some(err) => Poll::Ready(Err(err)),
        }
    }
}

struct Key(Instant, RawFd);

impl cmp::PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl cmp::Eq for Key {}

impl cmp::PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Key {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.0.cmp(&other.0) {
            cmp::Ordering::Equal => self.1.cmp(&other.1),
            cmp => cmp,
        }
    }
}

#[derive(Default)]
struct EpollData {
    timer: BTreeMap<Key, Arc<Mutex<EpollEvent>>>,
    waker: Option<Waker>,
}

pub struct Epoll {
    epfd: OwnedFd,
    data: Mutex<EpollData>,
}

impl Epoll {
    pub fn new() -> Result<Self, OsError> {
        unsafe {
            match libc::epoll_create1(libc::EPOLL_CLOEXEC) {
                -1 => Err(OsError::last()),
                epfd => Ok(Epoll {
                    epfd: OwnedFd::from_raw_fd(epfd),
                    data: Mutex::default(),
                }),
            }
        }
    }

    fn epoll_ctl(&self, soc: &ConnectedSocket, op: i32, ptr: u64) {
        let mut event = libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLOUT) as u32,
            u64: ptr,
        };
        unsafe {
            match libc::epoll_ctl(self.epfd.as_raw_fd(), op, soc.as_raw_fd(), &mut event) {
                -1 => {}
                0 => {}
                _ => unreachable!(),
            }
        }
    }

    pub(super) fn register_socket(&self, soc: ConnectedSocket, ctx: &IoContext) -> AsyncSocket {
        let soc = AsyncSocket::new(ctx, soc);
        self.epoll_ctl(soc.as_socket(), libc::EPOLL_CTL_ADD, soc.as_epoll_ptr());
        soc
    }

    pub(super) fn deregister_socket(&self, soc: &ConnectedSocket) {
        self.epoll_ctl(soc, libc::EPOLL_CTL_DEL, 0)
    }

    fn epoll_timeout(&self) -> i32 {
        let now = Instant::now();
        let data = self.data.lock().unwrap();
        if let Some((key, _)) = data.timer.first_key_value() {
            let millis = (key.0 - now).as_millis();
            if millis > i32::MAX as u128 {
                -1
            } else {
                millis as i32
            }
        } else {
            -1
        }
    }

    fn timer_cancel(&self, ctx: &mut Context) {
        let mut data = self.data.lock().unwrap();
        data.waker = Some(ctx.waker().clone());
    }

    pub fn poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        const EVENTLEN: usize = 128;
        let mut events = MaybeUninit::<[libc::epoll_event; EVENTLEN]>::uninit();
        unsafe {
            match libc::epoll_wait(
                self.epfd.as_raw_fd(),
                events.as_mut_ptr().cast(),
                EVENTLEN as i32,
                self.epoll_timeout(),
            ) {
                -1 => Poll::Ready(Err(OsError::last())),
                0 => Poll::Ready(Ok(())),
                len => {
                    self.timer_cancel(ctx);
                    let len = len as usize;
                    let events = events.assume_init();
                    for ev in &events[0..len] {
                        AsyncSocket::epoll_op(ev.u64, |op| {
                            if (ev.events & (libc::EPOLLERR | libc::EPOLLHUP) as u32) != 0 {
                                op.read_op = Some(OsError::OPERATION_CANCELED);
                                op.write_op = Some(OsError::OPERATION_CANCELED);
                            } else {
                                if (ev.events & libc::EPOLLIN as u32) != 0 {
                                    match op.read_op.take() {
                                        None => op.read_op = Some(OsError::READY),
                                        Some(OsError::READY) => op.read_op = Some(OsError::READY),
                                        Some(_) => unreachable!(),
                                    }
                                }
                                if (ev.events & libc::EPOLLOUT as u32) != 0 {
                                    match op.write_op.take() {
                                        None => op.write_op = Some(OsError::READY),
                                        Some(OsError::READY) => op.write_op = Some(OsError::READY),
                                        Some(_) => unreachable!(),
                                    }
                                }
                            }
                            op.waker.take()
                        });
                    }
                    Poll::Pending
                }
            }
        }
    }
}
