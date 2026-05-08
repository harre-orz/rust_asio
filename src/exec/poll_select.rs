use super::{Event, Interrupter};
use crate::error::{OsError, Result};
use crate::exec::event::{Deadline, EventScheduler};
use crate::socket::Socket;
use std::ptr;
use std::task::Poll;

#[cfg(unix)]
mod ffi {
    use crate::error::{OsError, Result};
    use crate::exec::event::Event;
    use crate::socket::Fd;
    use std::mem::MaybeUninit;
    use std::sync::Mutex;
    use std::{cmp, ptr};

    struct Inner {
        fds: Vec<(libc::c_int, Event)>,
        nfds: i32,
        rfds: libc::fd_set,
        wfds: libc::fd_set,
    }

    pub(crate) struct SelectImpl(Mutex<Inner>);

    impl SelectImpl {
        pub(super) fn new(fd: &Fd) -> Self {
            let mut fds = Vec::new();
            let mut rfds = MaybeUninit::<libc::fd_set>::uninit();
            let mut wfds = MaybeUninit::<libc::fd_set>::uninit();
            unsafe {
                let fd = fd.as_raw_fd();
                libc::FD_ZERO(rfds.as_mut_ptr());
                libc::FD_ZERO(wfds.as_mut_ptr());
                libc::FD_SET(fd, rfds.as_mut_ptr());
                Self(Mutex::new(Inner {
                    fds: fds,
                    nfds: fd + 1,
                    rfds: rfds.assume_init(),
                    wfds: wfds.assume_init(),
                }))
            }
        }

        pub(super) fn register(&self, soc: &Fd, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            let fd = unsafe { soc.as_raw_fd() };
            inner.nfds = cmp::max(inner.nfds, fd + 1);
            inner.fds.push((fd, event.clone()));
        }

        pub(super) fn deregister(&self, soc: &Fd) {
            let mut nfds = 0;
            let mut inner = self.0.lock().unwrap();
            let fd = unsafe { soc.as_raw_fd() };
            let mut i = 0;
            while i < inner.fds.len() {
                if inner.fds[i].0 == fd {
                    inner.fds.remove(i);
                } else {
                    nfds = cmp::max(nfds, inner.fds[i].0 + 1);
                    i += 1
                }
            }
            inner.nfds = nfds
        }

        pub(super) fn add_read_event(&self, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            unsafe { libc::FD_SET(event.as_raw_fd(), &mut inner.rfds) }
        }

        pub(super) fn del_read_event(&self, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            unsafe { libc::FD_CLR(event.as_raw_fd(), &mut inner.rfds) }
        }

        pub(super) fn add_write_event(&self, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            unsafe { libc::FD_SET(event.as_raw_fd(), &mut inner.wfds) }
        }

        pub(super) fn del_write_event(&self, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            unsafe { libc::FD_CLR(event.as_raw_fd(), &mut inner.wfds) }
        }

        pub(super) fn select(
            &self,
            mut tv: libc::timeval,
        ) -> Result<(Vec<(libc::c_int, Event)>, libc::fd_set, libc::fd_set)> {
            unsafe {
                let (nfds, mut rfds, mut wfds, fds) = {
                    let mut inner = self.0.lock().unwrap();
                    (inner.nfds, inner.rfds, inner.wfds, inner.fds.clone())
                };
                match libc::select(nfds, &mut rfds, &mut wfds, ptr::null_mut(), &mut tv) {
                    -1 => Err(OsError::last()),
                    _ => Ok((fds, rfds, wfds)),
                }
            }
        }
    }

    pub(super) fn is_set(fd: libc::c_int, fds: &libc::fd_set) -> bool {
        unsafe { libc::FD_ISSET(fd, fds) }
    }
}

#[cfg(windows)]
mod ffi {
    use std::sync::Mutex;
    use windows_sys::Win32::Networking::WinSock;
    use crate::exec::event::Event;

    struct Inner {
        fds: Vec<(WinSock::SOCKET, Event)>,
        rfds: WinSock::FD_SET,
        wfds: WinSock::FD_SET,
    }

    pub(super) struct SelectImpl(Mutex<Inner>);

    impl SelectImpl {
        pub(super) fn new(soc: WinSock::SOCKET) -> Self {
            Self(Mutex::new(
                Inner {
                    fds: Vec::new(),
                    rfds: Default::default(),
                    wfds: Default::default(),
                }
            ))
        }
    }
}

pub(super) struct Select {
    pub(super) intr: Interrupter,
    intr_event: Event,
    select: ffi::SelectImpl,
}

impl Select {
    pub(super) fn new() -> Result<Self> {
        let intr = Interrupter::new()?;
        let intr_event = Event::new(intr.as_fd());
        let select = ffi::SelectImpl::new(intr.as_fd());
        Ok(Self {
            intr: intr,
            intr_event: intr_event,
            select: select,
        })
    }

    pub(super) fn register_soc(&self, soc: &Socket, event: &Event) {
        self.select.register(soc.as_fd(), event)
    }

    pub(super) fn deregister_soc(&self, soc: &Socket) {
        self.select.deregister(soc.as_fd())
    }

    pub(super) fn add_read_event(&self, event: &Event) {
        self.select.add_read_event(event);
    }

    fn del_read_event(&self, event: &Event) {
        self.select.del_read_event(event)
    }

    pub(super) fn add_write_event(&self, event: &Event) {
        self.select.add_write_event(event)
    }

    fn del_write_event(&self, event: &Event) {
        self.select.del_write_event(event)
    }

    pub(super) fn poll(&self, scheduler: &EventScheduler) -> Poll<OsError> {
        let mut wakers = Vec::new();
        loop {
            match self.select.select(self.intr.timeout_select()) {
                Err(OsError::INTERRUPTED) => continue,
                Err(err) => return Poll::Ready(err),
                Ok((fds, rfds, wfds)) => {
                    let now = Deadline::now();
                    for (fd, event) in fds {
                        let mut readable = true;
                        let mut writable = true;
                        if ffi::is_set(fd, &rfds) {
                            if ptr::addr_eq(&self.intr_event, &event) {
                                self.intr.update_event();
                                continue;
                            }
                            self.del_read_event(&event);
                            readable = true;
                        }
                        if ffi::is_set(fd, &wfds) {
                            self.del_write_event(&event);
                            writable = true;
                        }
                        if readable || writable {
                            event.ready(readable, writable, &mut wakers);
                            scheduler.update_event(&event, now, &mut wakers);
                        }
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
