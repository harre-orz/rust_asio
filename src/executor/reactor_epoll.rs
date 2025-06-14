use super::{Event, EventScheduler, Intr};
use crate::error::OsError;
use crate::socket::ffi::Socket;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd};
use std::ptr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Instant;

mod ffi {
    use super::*;
    use std::os::fd::{AsRawFd, FromRawFd, RawFd};

    pub fn epoll_create() -> Result<OwnedFd, OsError> {
        match unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) } {
            -1 => Err(unsafe { OsError::last() }),
            fd => Ok(unsafe { OwnedFd::from_raw_fd(fd) }),
        }
    }

    pub fn epoll_add(epfd: &OwnedFd, soc: RawFd, events: i32, event: &Arc<Mutex<Event>>) {
        let mut event = libc::epoll_event {
            events: events as u32,
            u64: Arc::as_ptr(event) as u64,
        };
        match unsafe { libc::epoll_ctl(epfd.as_raw_fd(), libc::EPOLL_CTL_ADD, soc, &mut event) } {
            -1 => panic!(),
            0 => return,
            _ => unreachable!(),
        }
    }

    pub fn epoll_del(epfd: &OwnedFd, soc: RawFd) {
        let mut event = libc::epoll_event { events: 0, u64: 0 };
        match unsafe { libc::epoll_ctl(epfd.as_raw_fd(), libc::EPOLL_CTL_DEL, soc, &mut event) } {
            -1 => panic!(),
            0 => return,
            _ => unreachable!(),
        }
    }
}

struct Inner {
    waker: Option<Waker>,
    events: EventScheduler,
}

pub struct Epoll {
    epfd: OwnedFd,
    intr: Intr,
    data: Mutex<Inner>,
}

impl Drop for Epoll {
    fn drop(&mut self) {
        ffi::epoll_del(&self.epfd, self.intr.as_raw_fd())
    }
}

impl Epoll {
    pub fn new() -> Result<Self, OsError> {
        let epfd = ffi::epoll_create()?;
        let intr = Intr::new(Event::new())?;
        ffi::epoll_add(&epfd, intr.as_raw_fd(), libc::EPOLLIN, &intr.event);
        Ok(Epoll {
            epfd: epfd,
            intr: intr,
            data: Mutex::new(Inner {
                waker: None,
                events: EventScheduler::new(),
            }),
        })
    }

    pub fn register_socket(&self, soc: &Socket) -> Arc<Mutex<Event>> {
        let event = Event::new();
        ffi::epoll_add(
            &self.epfd,
            soc.as_raw_fd(),
            libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET,
            &event,
        );
        let mut data = self.data.lock().unwrap();
        data.events.insert(event.clone());
        event
    }

    pub fn deregister_socket(&self, soc: &Socket, event: &Arc<Mutex<Event>>) {
        ffi::epoll_del(&self.epfd, soc.as_raw_fd());
        let mut data = self.data.lock().unwrap();
        data.events.remove(event)
    }

    pub fn stop_request(&self) {
        self.intr.wake_up_now();
        let mut wakers = Vec::new();
        for event in {
            let data = self.data.lock().unwrap();
            data.events.collect()
        } {
            let mut event = event.lock().unwrap();
            event.read_result(Err(OsError::OPERATION_CANCELED), &mut wakers);
            event.write_result(Err(OsError::OPERATION_CANCELED), &mut wakers);
        }
        for waker in wakers {
            waker.wake()
        }
    }

    pub fn update_schedule(&self, event: &Arc<Mutex<Event>>, cto: Instant) {
        if {
            let mut data = self.data.lock().unwrap();
            data.events.update_deadline(event, cto)
        } {
            self.intr.wake_up_alarm(cto)
        }
    }

    pub fn ready_poll(&self) {
        if let Some(waker) = {
            let mut data = self.data.lock().unwrap();
            data.waker.take()
        } {
            waker.wake()
        }
    }

    pub fn poll(&self, ctx: &mut Context) -> Poll<Result<(), OsError>> {
        loop {
            const EVENTLEN: usize = 128;
            let mut events = MaybeUninit::<[libc::epoll_event; EVENTLEN]>::uninit();
            match unsafe {
                libc::epoll_wait(
                    self.epfd.as_raw_fd(),
                    events.as_mut_ptr().cast(),
                    EVENTLEN as i32,
                    self.intr.as_timeout_epoll(),
                )
            } {
                -1 => match unsafe { OsError::last() } {
                    OsError::INTERRUPTED => {}
                    err => return Poll::Ready(Err(err)),
                },
                len => {
                    let mut wakers = Vec::new();
                    let events = unsafe { events.assume_init() };
                    for ev in &events[..len as usize] {
                        let event = unsafe { Arc::from_raw(ev.u64 as *mut Mutex<Event>) };
                        if ptr::addr_eq(&event, &self.intr.event) {
                            self.intr.read();
                            continue;
                        }
                        let mut event = event.lock().unwrap();
                        if (ev.events & (libc::EPOLLERR | libc::EPOLLHUP) as u32) != 0 {
                            let err = OsError::OPERATION_CANCELED;
                            event.read_result(Err(err), &mut wakers);
                            event.write_result(Err(err), &mut wakers);
                        } else {
                            if (ev.events & libc::EPOLLIN as u32) != 0 {
                                event.read_result(Ok(()), &mut wakers);
                            }
                            if (ev.events & libc::EPOLLOUT as u32) != 0 {
                                event.write_result(Ok(()), &mut wakers);
                            }
                        }
                    }

                    let is_empty = wakers.is_empty();
                    for event in {
                        let now = Instant::now();
                        let mut data = self.data.lock().unwrap();
                        if !is_empty {
                            data.waker = Some(ctx.waker().clone());
                        }
                        data.events.timed_out(now)
                    } {
                        let mut event = event.lock().unwrap();
                        event.read_result(Err(OsError::OPERATION_CANCELED), &mut wakers);
                        event.write_result(Err(OsError::OPERATION_CANCELED), &mut wakers);
                    }
                    if wakers.is_empty() {
                        continue;
                    }
                    if is_empty {
                        let mut data = self.data.lock().unwrap();
                        data.waker = Some(ctx.waker().clone());
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
