use super::{Event, EventScheduler, Interrupter};
use crate::error::{OsError, Result};
use crate::exec::event::Deadline;
use crate::socket::{Fd, Socket};
use std::ptr;
use std::sync::Mutex;
use std::task::Poll;

mod ffi {
    use super::Event;
    use crate::error::{OsError, Result};
    use crate::socket::Fd;
    use std::mem::MaybeUninit;

    pub(super) fn kqueue() -> Result<Fd> {
        unsafe {
            match libc::kqueue() {
                -1 => Err(OsError::last()),
                fd => Ok(Fd::new_unchecked(fd)),
            }
        }
    }

    pub(super) const fn kevent_set(
        soc: &Fd,
        filter: i16,
        flags: u16,
        event: &Event,
    ) -> libc::kevent {
        unsafe {
            libc::kevent {
                ident: soc.as_raw_fd() as libc::uintptr_t,
                filter: filter,
                flags: flags,
                fflags: 0,
                data: 0,
                udata: event.as_raw(),
            }
        }
    }

    pub(super) fn kevent(
        kq: &Fd,
        changes: &[libc::kevent],
        mut timeout: libc::timespec,
    ) -> Result<(Box<[libc::kevent]>, usize)> {
        let mut kevents: Box<[MaybeUninit<libc::kevent>]> =
            Box::<[libc::kevent]>::new_uninit_slice(changes.len());
        unsafe {
            match libc::kevent(
                kq.as_raw_fd(),
                changes.as_ptr(),
                changes.len() as libc::c_int,
                kevents[0].as_mut_ptr(),
                kevents.len() as libc::c_int,
                &mut timeout,
            ) {
                -1 => Err(OsError::last()),
                len => Ok((kevents.assume_init(), len as usize)),
            }
        }
    }
}

pub(crate) struct Kqueue {
    kq: Fd,
    kevents: Mutex<Vec<libc::kevent>>,
    pub(super) intr: Interrupter,
    intr_event: Event,
}

impl Kqueue {
    pub(crate) fn new() -> Result<Self> {
        let kq = ffi::kqueue()?;
        let intr = Interrupter::new()?;
        let intr_event = unsafe { Event::new(intr.as_raw_fd()) };
        let mut kevents = Vec::new();
        kevents.push(ffi::kevent_set(
            intr.as_fd(),
            libc::EVFILT_READ,
            libc::EV_ADD | libc::EV_ENABLE,
            &intr_event,
        ));
        Ok(Self {
            kq: kq,
            kevents: Mutex::new(kevents),
            intr: intr,
            intr_event: intr_event,
        })
    }

    pub(crate) fn register_soc(&self, soc: &Socket, event: &Event) {
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(ffi::kevent_set(
            soc.as_fd(),
            libc::EVFILT_READ,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
        kevents.push(ffi::kevent_set(
            soc.as_fd(),
            libc::EVFILT_WRITE,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
    }

    pub(crate) fn deregister_soc(&self, soc: &Socket) {
        let mut kevents = self.kevents.lock().unwrap();
        let mut i = 0;
        while i < kevents.len() {
            if kevents[i].ident == unsafe { soc.as_raw_fd() as libc::uintptr_t } {
                kevents.remove(i);
            } else {
                i += 1
            }
        }
    }

    pub(super) fn add_read_event(&self, _: &Event) {}

    fn del_read_event(&self, _: &Event) {}

    pub(super) fn add_write_event(&self, _: &Event) {}

    fn del_write_event(&self, _: &Event) {}

    pub(super) fn poll(&self, scheduler: &EventScheduler) -> Poll<OsError> {
        let mut wakers = Vec::new();
        let changes = {
            let kevents = self.kevents.lock().unwrap();
            kevents.clone()
        };
        loop {
            match ffi::kevent(&self.kq, changes.as_slice(), self.intr.timeout_kqueue()) {
                Err(OsError::INTERRUPTED) => continue,
                Err(err) => return Poll::Ready(err),
                Ok((kevents, len)) => {
                    let now = Deadline::now();
                    for kev in &kevents[..len] {
                        let event = unsafe { Event::from_raw_ptr(kev.udata.cast()) };
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event();
                            continue;
                        }
                        if kev.filter == libc::EVFILT_READ {
                            event.ready(true, false, &mut wakers);
                        }
                        if kev.filter == libc::EVFILT_WRITE {
                            event.ready(false, true, &mut wakers);
                        }
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
