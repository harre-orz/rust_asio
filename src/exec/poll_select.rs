use super::{Event, Interrupter};
use crate::error::{OsError, Result};
use crate::exec::event::{Deadline, EventScheduler};
use crate::socket::Socket;
use std::ptr;
use std::task::Poll;

#[cfg(unix)]
mod ffi {
    use super::Socket;
    use crate::error::{OsError, Result};
    use crate::exec::event::Event;
    use std::mem::MaybeUninit;
    use std::sync::Mutex;
    use std::time::Duration;
    use std::{cmp, ptr};

    struct Inner {
        fds: Vec<(libc::c_int, Event)>,
        nfds: i32,
        rfds: libc::fd_set,
        wfds: libc::fd_set,
    }

    pub(crate) struct SelectImpl(Mutex<Inner>);

    impl SelectImpl {
        pub(super) fn new(raw_fd: libc::c_int) -> Self {
            let fds = Vec::new();
            let mut rfds = MaybeUninit::<libc::fd_set>::uninit();
            let mut wfds = MaybeUninit::<libc::fd_set>::uninit();
            unsafe {
                libc::FD_ZERO(rfds.as_mut_ptr());
                libc::FD_ZERO(wfds.as_mut_ptr());
                libc::FD_SET(raw_fd, rfds.as_mut_ptr());
                Self(Mutex::new(Inner {
                    fds: fds,
                    nfds: raw_fd + 1,
                    rfds: rfds.assume_init(),
                    wfds: wfds.assume_init(),
                }))
            }
        }

        pub(super) fn register(&self, soc: &Socket, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            let fd = unsafe { soc.as_raw_fd() };
            inner.nfds = cmp::max(inner.nfds, fd + 1);
            inner.fds.push((fd, event.clone()));
        }

        pub(super) fn deregister(&self, soc: &Socket) {
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
            unsafe { libc::FD_SET(event.as_native_handle(), &mut inner.rfds) }
        }

        pub(super) fn del_read_event(&self, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            unsafe { libc::FD_CLR(event.as_native_handle(), &mut inner.rfds) }
        }

        pub(super) fn add_write_event(&self, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            unsafe { libc::FD_SET(event.as_native_handle(), &mut inner.wfds) }
        }

        pub(super) fn del_write_event(&self, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            unsafe { libc::FD_CLR(event.as_native_handle(), &mut inner.wfds) }
        }

        pub(super) fn select(
            &self,
            timeout: Duration,
        ) -> Result<(Vec<(libc::c_int, Event)>, libc::fd_set, libc::fd_set)> {
            unsafe {
                let (nfds, mut rfds, mut wfds, fds) = {
                    let inner = self.0.lock().unwrap();
                    (inner.nfds, inner.rfds, inner.wfds, inner.fds.clone())
                };
                let mut tv = libc::timeval {
                    tv_sec: timeout.as_secs() as libc::time_t,
                    #[cfg(target_os = "linux")]
                    tv_usec: timeout.subsec_micros() as libc::c_long,
                    #[cfg(target_os = "macos")]
                    tv_usec: timeout.subsec_micros() as libc::suseconds_t,
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
    use crate::error::{OsError, Result};
    use crate::exec::event::Event;
    use crate::socket::Socket;
    use std::ptr;
    use std::sync::Mutex;
    use std::time::Duration;
    use windows_sys::Win32::Networking::WinSock;

    struct Inner {
        fds: Vec<(WinSock::SOCKET, Event)>,
        rfds: WinSock::FD_SET,
        wfds: WinSock::FD_SET,
    }

    pub(super) struct SelectImpl(Mutex<Inner>);

    impl SelectImpl {
        pub(super) fn new(soc: WinSock::SOCKET) -> Self {
            Self(Mutex::new(Inner {
                fds: Vec::new(),
                rfds: Default::default(),
                wfds: Default::default(),
            }))
        }

        pub(super) fn register(&self, soc: &Socket, event: &Event) {
            let mut inner = self.0.lock().unwrap();
            inner
                .fds
                .push((unsafe { soc.as_raw_socket() }, event.clone()));
        }

        pub(super) fn deregister(&self, soc: &Socket) {
            let mut inner = self.0.lock().unwrap();
            let mut i = 0;
            while i < inner.fds.len() {
                if inner.fds[i].0 == unsafe { soc.as_raw_socket() } {
                    inner.fds.remove(i);
                } else {
                    i += 1
                }
            }
        }

        pub(super) fn add_read_event(&self, event: &Event) {}

        pub(super) fn del_read_event(&self, event: &Event) {}

        pub(super) fn add_write_event(&self, event: &Event) {}

        pub(super) fn del_write_event(&self, event: &Event) {}

        pub(super) fn select(
            &self,
            timeout: Duration,
        ) -> Result<(
            Vec<(WinSock::SOCKET, Event)>,
            WinSock::FD_SET,
            WinSock::FD_SET,
        )> {
            unsafe {
                let (mut rfds, mut wfds, fds) = {
                    let mut inner = self.0.lock().unwrap();
                    (inner.rfds, inner.wfds, inner.fds.clone())
                };
                let mut tv = WinSock::TIMEVAL {
                    tv_sec: timeout.as_secs() as i32,
                    tv_usec: timeout.subsec_micros() as i32,
                };
                match WinSock::select(0, &mut rfds, &mut wfds, ptr::null_mut(), &mut tv) {
                    -1 => Err(OsError::last()),
                    _ => Err(OsError::last()),
                }
            }
        }
    }

    pub(super) fn is_set(soc: WinSock::SOCKET, fds: &WinSock::FD_SET) -> bool {
        false
    }
}

pub(super) struct Select {
    pub(super) intr: Interrupter,
    intr_event: Event,
    select_impl: ffi::SelectImpl,
}

impl Select {
    pub(super) fn new() -> Result<Self> {
        let intr = Interrupter::new()?;
        let handle = unsafe { intr.as_native_handle() };
        let intr_event = Event::new(handle);
        let select_impl = ffi::SelectImpl::new(handle);
        Ok(Self {
            intr: intr,
            intr_event: intr_event,
            select_impl: select_impl,
        })
    }

    pub(super) fn register_soc(&self, soc: &Socket, event: &Event) {
        self.select_impl.register(soc, event)
    }

    pub(super) fn deregister_soc(&self, soc: &Socket) {
        self.select_impl.deregister(soc)
    }

    pub(super) fn add_read_event(&self, event: &Event) {
        self.select_impl.add_read_event(event);
    }

    fn del_read_event(&self, event: &Event) {
        self.select_impl.del_read_event(event)
    }

    pub(super) fn add_write_event(&self, event: &Event) {
        self.select_impl.add_write_event(event)
    }

    fn del_write_event(&self, event: &Event) {
        self.select_impl.del_write_event(event)
    }

    pub(super) fn poll(&self, scheduler: &EventScheduler) -> Poll<OsError> {
        let mut wakers = Vec::new();
        loop {
            match self.select_impl.select(self.intr.timeout()) {
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
