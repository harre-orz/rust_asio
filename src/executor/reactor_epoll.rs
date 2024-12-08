use super::{Event, Intr};
use crate::error::OsError;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::mem::MaybeUninit;
use std::ptr;
use std::time::Instant;
use std::collections::{HashSet, BTreeMap};
use std::hash;

mod ffi {
    use super::*;
    use std::os::fd::FromRawFd;

    pub fn epoll_create() -> Result<OwnedFd, OsError> {
        match unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) } {
            -1 => Err(unsafe { OsError::last() }),
            fd => Ok(unsafe { OwnedFd::from_raw_fd(fd) }),
        }
    }

    pub fn epoll_add<F>(epfd: &OwnedFd, fd: &F, events: i32, event: &Arc<Mutex<Event>>)
    where
        F: AsRawFd,
    {
        let mut event = libc::epoll_event {
            events: events as u32,
	    u64: Arc::as_ptr(event) as u64,
        };
        match unsafe {
            libc::epoll_ctl(
                epfd.as_raw_fd(),
                libc::EPOLL_CTL_ADD,
                fd.as_raw_fd(),
                &mut event,
            )
        } {
            -1 => panic!(),
            0 => return,
            _ => unreachable!(),
        }
    }

    pub fn epoll_del<F>(epfd: &OwnedFd, fd: &F)
    where
        F: AsRawFd,
    {
        let mut event = libc::epoll_event { events: 0, u64: 0 };
        match unsafe {
            libc::epoll_ctl(
                epfd.as_raw_fd(),
                libc::EPOLL_CTL_DEL,
                fd.as_raw_fd(),
                &mut event,
            )
        } {
            -1 => panic!(),
            0 => return,
            _ => unreachable!(),
        }
    }
}

struct EventPtr(Arc<Mutex<Event>>);

impl PartialEq for EventPtr {
    fn eq(&self, other: &Self) -> bool {
	ptr::addr_eq(&self.0, &other.0)
    }
}

impl Eq for EventPtr {}

impl hash::Hash for EventPtr {
    fn hash<H>(&self, hasher: &mut H)
    where
	H: hash::Hasher
    {
	hasher.write_usize(Arc::as_ptr(&self.0) as usize)
    }
}

struct Inner {
    waker: Option<Waker>,
    events: HashSet<EventPtr>,
    scheduler: BTreeMap<Instant, Arc<Mutex<Event>>>,
}

pub struct Epoll {
    epfd: OwnedFd,
    intr: Intr,
    data: Mutex<Inner>,
}

impl Drop for Epoll {
    fn drop(&mut self) {
	ffi::epoll_del(&self.epfd, &self.intr)
    }
}

impl Epoll {
    pub fn new() -> Result<Self, OsError> {
	let epfd = ffi::epoll_create()?;
	let intr = Intr::new(Event::new())?;
	ffi::epoll_add(&epfd, &intr, libc::EPOLLIN, &intr.event);
	Ok(Epoll {
	    epfd: epfd,
	    intr: intr,
	    data: Mutex::new(Inner {
		waker: None,
		events: HashSet::new(),
		scheduler: BTreeMap::new(),
	    }),
	})
    }
    
    pub fn register_socket<F>(&self, soc: &F) -> Arc<Mutex<Event>>
    where
	F: AsRawFd,
    {
	let event = Event::new();
	ffi::epoll_add(
	    &self.epfd,
	    soc,
	    libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET,
	    &event,
	);
	let mut data = self.data.lock().unwrap();
	data.events.insert(EventPtr(event.clone()));
	event
    }

    pub fn deregister_socket<F>(&self, soc: &F, event: &Arc<Mutex<Event>>)
    where
	F: AsRawFd,
    {
	ffi::epoll_del(&self.epfd, soc);
	let mut data = self.data.lock().unwrap();
	data.events.remove(&EventPtr(event.clone()));
    }

    pub fn stop_request(&self) {
	self.intr.intr();
	let mut wakers = Vec::new();
	for event in {
	    let mut data = self.data.lock().unwrap();
	    data.events.iter().map(|ev| ev.0.clone()).collect::<Vec<Arc<Mutex<Event>>>>()
	} {
	    let mut event = event.lock().unwrap();
	    event.read_result(Err(OsError::OPERATION_CANCELED), &mut wakers);
	    event.write_result(Err(OsError::OPERATION_CANCELED), &mut wakers);
	}
	for waker in wakers {
	    waker.wake()
	}
    }

    pub fn wake(&self) {
	if let Some(waker) = {
	    let mut data = self.data.lock().unwrap();
	    data.waker.take()
	} {
	    waker.wake()
	}
    }
    
    pub fn reset_event(&self, event: Arc<Mutex<Event>>, time: Instant) {
	let mut data = self.data.lock().unwrap();
	data.scheduler.insert(time, event);
    }

    pub fn timed_out_events(&self, now: Instant) -> Vec<Arc<Mutex<Event>>> {
	Vec::new()
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
			    continue
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
		    for event in self.timed_out_events(Instant::now()) {
			let mut event = event.lock().unwrap();
			event.read_result(Err(OsError::OPERATION_CANCELED), &mut wakers);
			event.write_result(Err(OsError::OPERATION_CANCELED), &mut wakers);
		    }
		    if wakers.is_empty() {
			continue
		    }
		    for waker in wakers {
			waker.wake()
		    }
		    return Poll::Pending
		},
	    }
	}
    }
}
