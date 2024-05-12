use crate::error::OsError;
use crate::ffi::{ConnectedSocket, Timeout};
use libc;
use std::cmp;
use std::collections::{BTreeMap, HashMap};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Instant;

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
        match self.0.cmp(&other.0).reverse() {
            cmp::Ordering::Equal => self.1.cmp(&other.1),
            cmp => cmp,
        }
    }
}

#[derive(PartialEq, Eq)]
enum Mode {
    None,
    Read,
    Write,
}

pub(crate) struct EpollEvent {
    mode: Mode,
    waker: Option<Waker>,
    read_op: Option<OsError>,
    write_op: Option<OsError>,
}

impl EpollEvent {
    pub fn read_reset(
        event: Arc<Mutex<Self>>,
        epoll: &Epoll,
        soc: &ConnectedSocket,
        timeout: Timeout,
    ) {
        if let Some(waker) = {
            let mut event = event.lock().unwrap();
            event.mode = Mode::Read;
            event.waker.take()
        } {
            waker.wake();
        }
        if let Some(waker) = {
            let key = Key(timeout.into_expire(), soc.as_raw_fd());
            let mut data = epoll.data.lock().unwrap();
            data.timer.insert(key, event);
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
        event: Arc<Mutex<Self>>,
        epoll: &Epoll,
        soc: &ConnectedSocket,
        timeout: Timeout,
    ) {
        if let Some(waker) = {
            let mut event = event.lock().unwrap();
            event.mode = Mode::Write;
            event.waker.take()
        } {
            waker.wake();
        }
        if let Some(waker) = {
            let key = Key(timeout.into_expire(), soc.as_raw_fd());
            let mut data = epoll.data.lock().unwrap();
            data.timer.insert(key, event);
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

#[derive(Default)]
struct EpollData {
    waker: Option<Waker>,
    timer: BTreeMap<Key, Arc<Mutex<EpollEvent>>>,
    trash: HashMap<RawFd, Arc<Mutex<EpollEvent>>>,
}

pub(super) struct Epoll {
    epfd: OwnedFd,
    data: Mutex<EpollData>,
}

impl Epoll {
    pub fn new() -> Result<Self, OsError> {
        match unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) } {
            -1 => Err(unsafe { OsError::last() }),
            epfd => {
                let epfd = unsafe { OwnedFd::from_raw_fd(epfd) };
                Ok(Epoll {
                    epfd,
                    data: Mutex::default(),
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

    pub fn register_socket(&self, soc: &ConnectedSocket) -> Arc<Mutex<EpollEvent>> {
        let event = Arc::new(Mutex::new(EpollEvent {
            mode: Mode::None,
            waker: None,
            read_op: None,
            write_op: None,
        }));
        self.epoll_ctl(soc, libc::EPOLL_CTL_ADD, Arc::as_ptr(&event) as u64);
        event
    }

    pub fn deregister_socket(&self, soc: &ConnectedSocket) {
        self.epoll_ctl(soc, libc::EPOLL_CTL_DEL, 0);
        let mut temp = BTreeMap::new();
        let mut data = self.data.lock().unwrap();
        while let Some((key, val)) = data.timer.pop_first() {
            if key.1 == soc.as_raw_fd() {
                let _ = data.trash.insert(soc.as_raw_fd(), val);
            } else {
                temp.insert(key, val);
            }
        }
        data.timer.append(&mut temp);
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
            0
        }
    }

    fn time_expire(&self, ctx: &mut Context, events: &[libc::epoll_event]) {
        let mut trash = HashMap::new();
        let timeout = {
            let now = Instant::now();
            let mut data = self.data.lock().unwrap();
            data.waker = Some(ctx.waker().clone());
            let timeout = data.timer.split_off(&Key(now, 0));
            if timeout.is_empty() {
                return;
            }
            let mut temp = BTreeMap::new();
            while let Some((key, val)) = data.timer.pop_last() {
                let mut found = false;
                for ev in events {
                    if ev.u64 == Arc::as_ptr(&val) as u64 {
                        found = true;
                        let _ = trash.insert(key.1, val.clone());
                    }
                }
                if !found {
                    temp.insert(key, val);
                }
            }
            data.timer.append(&mut temp);
            timeout
        };
        for (key, val) in timeout {
            if let None = trash.get(&key.1) {
                let mut event = val.lock().unwrap();
                event.read_op = Some(OsError::OPERATION_CANCELED);
                event.write_op = Some(OsError::OPERATION_CANCELED);
                if let Some(waker) = event.waker.take() {
                    waker.wake()
                }
            }
        }
    }

    pub fn poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        let mut retry = true;
        loop {
            const EVENTLEN: usize = 128;
            let mut events = MaybeUninit::<[libc::epoll_event; EVENTLEN]>::uninit();
            let timeout = self.epoll_timeout();
            match unsafe {
                libc::epoll_wait(
                    self.epfd.as_raw_fd(),
                    events.as_mut_ptr().cast(),
                    EVENTLEN as i32,
                    timeout,
                )
            } {
                -1 => return Poll::Ready(Err(unsafe { OsError::last() })),
                0 => return Poll::Ready(Ok(())),
                len => {
                    let events = unsafe { events.assume_init() };
                    let events = &events[..len as usize];
                    for ev in events {
                        let event = unsafe { Arc::from_raw(ev.u64 as *const Mutex<EpollEvent>) };
                        if let Some(waker) = {
                            let mut event = event.lock().unwrap();
                            if (ev.events & libc::EPOLLIN as u32) != 0 {
                                event.read_op = Some(OsError::READY);
                                if event.mode == Mode::Read {
                                    event.mode = Mode::None;
                                    retry = false;
                                }
                            }
                            if (ev.events & libc::EPOLLOUT as u32) != 0 {
                                event.write_op = Some(OsError::READY);
                                if event.mode == Mode::Write {
                                    event.mode = Mode::None;
                                    retry = false;
                                }
                            }
                            event.waker.take()
                        } {
                            waker.wake();
                        }
                    }
                    if retry {
                        continue;
                    }
                    self.time_expire(ctx, events);
                    return Poll::Pending;
                }
            }
        }
    }
}

#[test]
fn test_ordering() {
    use std::time::Duration;

    let now = Instant::now();
    let mut data: BTreeMap<Key, i32> = BTreeMap::new();
    data.insert(Key(now - Duration::new(10, 0), 1), 1); // timeout
    data.insert(Key(now + Duration::new(10, 0), 2), 2);
    data.insert(Key(now - Duration::new(10, 0), 3), 3); // timeout
    data.insert(Key(now - Duration::new(10, 0), 4), 4); // timeout
    data.insert(Key(now + Duration::new(10, 0), 5), 5);

    let mut exp = data.split_off(&Key(now, 0));
    if let Some((key, _)) = exp.pop_last() {
        assert_eq!(key.1, 4);
    }
    if let Some((key, _)) = exp.pop_last() {
        assert_eq!(key.1, 3);
    }
    if let Some((key, _)) = exp.pop_last() {
        assert_eq!(key.1, 1);
    }
    assert_eq!(exp.is_empty(), true);
}
