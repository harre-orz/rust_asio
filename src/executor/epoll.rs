use super::{AsyncSocket, IoContext};
use crate::error::OsError;
use crate::ffi::ConnectedSocket;
use libc;
use std::mem::MaybeUninit;
use std::os::fd::{RawFd, AsRawFd, FromRawFd, OwnedFd};
use std::task::{Context, Poll, Waker};
use std::time::{Instant, Duration};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::cmp;

#[derive(PartialEq, Eq)]
enum Op {
    Read,
    Write,
}

struct Key(Instant, RawFd, Op);

impl cmp::PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1 && self.2 == other.2
    }
}

impl cmp::Eq for Key {
}

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

pub struct EpollEvent {
    read_op: Option<OsError>,
    read_waker: Option<Waker>,
    read_expire: Instant,
    write_op: Option<OsError>,
    write_waker: Option<Waker>,
    write_expire: Instant,
}

impl EpollEvent {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            read_op: None,
            read_waker: None,
            read_expire: now,
            write_op: None,
            write_waker: None,
            write_expire: now,
        }
    }

    pub fn read_reset(event: &Arc<Mutex<Self>>, soc: &ConnectedSocket, timeout: Duration, epoll: &Epoll) {
        let expire = Instant::now() + timeout;
        {
            let mut event = event.lock().unwrap();
            event.read_op = None;
            event.read_waker = None;
            event.read_expire = expire;
        }
        {
            let mut timer = epoll.timer.lock().unwrap();
            timer.insert(Key(expire, soc.as_raw_fd(), Op::Read), event.clone());
        }
    }

    pub fn write_reset(event: &Arc<Mutex<Self>>, soc: &ConnectedSocket, timeout: Duration, epoll: &Epoll) {
        let expire = Instant::now() + timeout;
        {
            let mut event = event.lock().unwrap();
            event.write_op = None;
            event.write_waker = None;
            event.write_expire = expire;
        }
        {
            let mut timer = epoll.timer.lock().unwrap();
            timer.insert(Key(expire, soc.as_raw_fd(), Op::Write), event.clone());
        }
    }

    pub fn read_poll(&mut self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        match self.read_op {
            Some(OsError::READY) => Poll::Ready(Ok(())),
            Some(err) => Poll::Ready(Err(err)),
            None => {
                self.read_waker = Some(ctx.waker().clone());
                Poll::Pending
            }
        }
    }

    pub fn write_poll(&mut self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        match self.write_op {
            Some(OsError::READY) => Poll::Ready(Ok(())),
            Some(err) => Poll::Ready(Err(err)),
            None => {
                self.write_waker = Some(ctx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub struct Epoll {
    epfd: OwnedFd,
    timer: Mutex<BTreeMap<Key, Arc<Mutex<EpollEvent>>>>,
}

impl Epoll {
    pub fn new() -> Result<Self, OsError> {
        unsafe {
            match libc::epoll_create1(libc::EPOLL_CLOEXEC) {
                -1 => Err(OsError::last()),
                epfd => Ok(Epoll {
                    epfd: OwnedFd::from_raw_fd(epfd),
                    timer: Mutex::default(),
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
        let time = self.timer.lock().unwrap();
        if let Some((key, _)) = time.first_key_value() {
            if key.0 < now {
                0
            } else {
                (key.0 - now).as_millis() as i32
            }
        } else {
            1000
        }
    }

    fn timer_cancel(&self, ctx: &mut Context) {
    }

    pub fn poll(&self, ctx: &mut Context) -> Poll<Result<usize, OsError>> {
        let mut events = MaybeUninit::<[libc::epoll_event; 128]>::uninit();
        unsafe {
            match libc::epoll_wait(self.epfd.as_raw_fd(), events.as_mut_ptr().cast(), 128, self.epoll_timeout()) {
                -1 => Poll::Ready(Err(OsError::last())),
                0 => {
                    println!("epoll");
                    ctx.waker().clone().wake();
                    Poll::Pending
                },
                len => {
                    let len = len as usize;
                    self.timer_cancel(ctx);
                    let events = events.assume_init();
                    for ev in &events[0..len] {
                        AsyncSocket::epoll_op(ev.u64, |op| {
                            if (ev.events & (libc::EPOLLERR | libc::EPOLLHUP) as u32) != 0 {
                                op.read_op = Some(OsError::CONNECTION_ABORTED);
                                if let Some(waker) = op.read_waker.take() {
                                    waker.wake()
                                }
                                op.write_op = Some(OsError::CONNECTION_ABORTED);
                                if let Some(waker) = op.write_waker.take() {
                                    waker.wake()
                                }
                            } else {
                                if (ev.events & libc::EPOLLIN as u32) != 0 {
                                    op.read_op = Some(OsError::CONNECTION_ABORTED);
                                    if let Some(waker) = op.read_waker.take() {
                                        waker.wake()
                                    }
                                }
                                if (ev.events & libc::EPOLLOUT as u32) != 0 {
                                    op.write_op = Some(OsError::CONNECTION_ABORTED);
                                    if let Some(waker) = op.write_waker.take() {
                                        waker.wake()
                                    }
                                }
                            }
                        });
                    }
                    Poll::Ready(Ok(len))
                }
            }
        }
    }
}
