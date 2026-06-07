use super::{Deadline, Event, EventScheduler, Interrupter};
use crate::error::{OsError, Result};
use crate::socket::{Fd, Socket};
use std::mem::MaybeUninit;
use std::ptr;
use std::task::Poll;

pub(super) fn epoll_create() -> Result<Fd> {
    unsafe {
        match libc::epoll_create1(libc::EPOLL_CLOEXEC) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::new_unchecked(fd)),
        }
    }
}

pub(super) fn epoll_add(epfd: &Fd, soc: &Fd, events: u32, event: &Event) {
    let mut event = libc::epoll_event {
        events: events,
        data: libc::epoll_data {
            ptr: event.as_raw_ptr().cast(),
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

pub(super) fn epoll_del(epfd: &Fd, soc: &Fd) {
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

pub(super) fn epoll_wait<const N: usize>(
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

pub(super) struct Epoll {
    epfd: Fd,
    pub(super) intr: Interrupter,
    intr_event: Event,
}

impl Drop for Epoll {
    fn drop(&mut self) {
        epoll_del(&self.epfd, self.intr.as_fd())
    }
}

impl Epoll {
    pub(super) fn new() -> Result<Self> {
        let epfd = epoll_create()?;
        let intr = Interrupter::new()?;
        #[cfg(unix)]
        let handle = unsafe { intr.as_native_handle() };
        let intr_event = Event::new(handle);
        epoll_add(&epfd, intr.as_fd(), libc::EPOLLIN, &intr_event);
        Ok(Epoll {
            epfd: epfd,
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(super) fn register_soc(&self, soc: &Socket, event: &Event) {
        epoll_add(
            &self.epfd,
            soc.as_fd(),
            libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET,
            event,
        )
    }

    pub(super) fn deregister_soc(&self, soc: &Socket) {
        epoll_del(&self.epfd, soc.as_fd());
    }

    pub(super) fn poll(&self, scheduler: &EventScheduler) -> Poll<OsError> {
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
                        let event = unsafe { Event::from_raw_ptr(eev.data.ptr.cast()) };
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
                        event.ready(readable, writable, &mut wakers);
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
