use super::{Deadline, Intr, Scheduler};
use crate::error::{OsError, Result};
use crate::primitive::{Fd, Signal, Socket};
use std::mem;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Debug)]
enum EventOp {
    Ready,
    Pending(Waker),
    Canceled,
    Neutral,
}

struct Op {
    readable_op: EventOp,
    writable_op: EventOp,
}

#[derive(Clone)]
pub(crate) struct Kevent(Arc<(Socket, Mutex<Op>)>);

impl Kevent {
    pub fn new(soc: Socket) -> Self {
        Self(Arc::new((
            soc,
            Mutex::new(Op {
                readable_op: EventOp::Ready,
                writable_op: EventOp::Ready,
            }),
        )))
    }

    pub fn as_socket(&self) -> &Socket {
        &self.0.0
    }

    pub fn read_poll(&self, ctx: &mut Context) -> Poll<Result<()>> {
        // match self.readable_op {
        //     EventOp::Ready => {
        //         self.readable_op = EventOp::Neutral;
        //         Poll::Ready(Ok(()))
        //     }
        //     EventOp::Pending(_) => Poll::Pending,
        //     EventOp::Canceled => {
        //         self.readable_op = EventOp::Neutral;
        //         Poll::Ready(Err(OsError::OPERATION_CANCELED))
        //     }
        //     EventOp::Neutral => {
        //         self.readable_op = EventOp::Pending(ctx.waker().clone());
        //         Poll::Pending
        //     }
        // }
        Poll::Pending
    }

    pub fn write_poll(&self, ctx: &mut Context) -> Poll<Result<()>> {
        // match self.writable_op {
        //     EventOp::Neutral => {
        //         self.writable_op = EventOp::Pending(ctx.waker().clone());
        //         Poll::Pending
        //     }
        //     EventOp::Pending(_) => Poll::Pending,
        //     EventOp::Ready => {
        //         self.writable_op = EventOp::Neutral;
        //         Poll::Ready(Ok(()))
        //     }
        //     EventOp::Canceled => {
        //         self.writable_op = EventOp::Neutral;
        //         Poll::Ready(Err(OsError::OPERATION_CANCELED))
        //     }
        // }
        Poll::Pending
    }

    pub fn cancel(&self, vec: &mut Vec<Waker>) {
        // let mut event_op = EventOp::Canceled;
        // mem::swap(&mut event_op, &mut self.readable_op);
        // if let EventOp::Pending(waker) = event_op {
        //     vec.push(waker);
        // }
        // let mut event_op = EventOp::Canceled;
        // mem::swap(&mut event_op, &mut self.writable_op);
        // if let EventOp::Pending(waker) = event_op {
        //     vec.push(waker);
        // }
    }
}

fn kqueue() -> Result<Fd> {
    unsafe {
        match libc::kqueue() {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

trait AsKeventIdent {
    fn ident(&self) -> libc::uintptr_t;
}

impl AsKeventIdent for Fd {
    fn ident(&self) -> libc::uintptr_t {
        unsafe { self.as_raw_fd() as libc::uintptr_t }
    }
}

impl AsKeventIdent for Signal {
    fn ident(&self) -> libc::uintptr_t {
        self.number() as libc::uintptr_t
    }
}

fn kevent_set<T>(data: &T, filter: i16, flags: u16, event: &Kevent) -> libc::kevent
where
    T: AsKeventIdent,
{
    unsafe {
        libc::kevent {
            ident: data.ident(),
            filter: filter,
            flags: flags,
            fflags: 0,
            data: 0,
            udata: Arc::into_raw(event.0.clone()).cast_mut().cast(),
        }
    }
}

fn kevent(
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

pub(in super::super) struct Kqueue {
    kq: Fd,
    kevents: Mutex<Vec<libc::kevent>>,
    intr: Intr,
    intr_event: Kevent,
}

impl Kqueue {
    pub fn new() -> Result<Self> {
        let kq = kqueue()?;
        let (intr, fd) = Intr::new()?;
        let intr_event = Kevent::new(Socket(fd));
        let mut kevents = Vec::new();
        kevents.push(kevent_set(
            &intr_event.0.0.0,
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

    // pub fn add_socket(&self, event: &crate::core::poll::epoll::EpollEvent) {
    //     let flags = libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET;
    //     crate::core::poll::epoll::epoll_add(
    //         &self.epfd,
    //         event,
    //         flags,
    //     )
    // }
    //
    // pub fn del_socket(&self, event: &crate::core::poll::epoll::EpollEvent) {
    //     crate::core::poll::epoll::epoll_del(&self.epfd, event)
    // }

    pub(crate) fn add_socket(&self, event: &Kevent) {
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            &event.0.0.0,
            libc::EVFILT_READ,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
        kevents.push(kevent_set(
            &event.0.0.0,
            libc::EVFILT_WRITE,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
    }

    pub(crate) fn del_socket(&self, event: &Kevent) {
        let mut kevents = self.kevents.lock().unwrap();
        let mut i = 0;
        while i < kevents.len() {
            if kevents[i].ident == event.as_socket().0.ident() {
                kevents.remove(i);
            } else {
                i += 1
            }
        }
    }

    pub fn wake_up_now(&self) {
        self.intr.wake_up_now(&self.intr_event.as_socket().0)
    }

    pub fn add_signal(&self, sig: Signal, event: &Kevent) {
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            &sig,
            libc::EVFILT_SIGNAL,
            libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            event,
        ));
    }

    pub(super) fn del_signal(&self, sig: Signal, event: &Kevent) {
        let mut kevents = self.kevents.lock().unwrap();
        kevents.push(kevent_set(
            &sig,
            libc::EVFILT_SIGNAL,
            libc::EV_DELETE,
            event,
        ));
    }

    pub fn poll(&self, scheduler: &Scheduler) -> Poll<OsError> {
        let mut wakers = Vec::new();
        let changes = {
            let kevents = self.kevents.lock().unwrap();
            kevents.clone()
        };
        loop {
            match kevent(&self.kq, changes.as_slice(), self.intr.timeout_kqueue()) {
                Err(OsError::INTERRUPTED) => continue,
                Err(err) => return Poll::Ready(err),
                Ok((kevents, len)) => {
                    let now = Deadline::now();
                    for kev in &kevents[..len] {
                        let event = Kevent(unsafe { Arc::from_raw(kev.udata.cast()) });
                        if ptr::addr_eq(&self.intr_event, &event) {
                            self.intr.update_event(&self.intr_event.0.0.0);
                            continue;
                        }
                        if kev.filter == libc::EVFILT_READ {
                            let mut event = event.0.1.lock().unwrap();
                            let mut event_op = EventOp::Ready;
                            mem::swap(&mut event_op, &mut event.readable_op);
                            if let EventOp::Pending(waker) = event_op {
                                wakers.push(waker);
                            }
                        }
                        if kev.filter == libc::EVFILT_WRITE {
                            let mut event = event.0.1.lock().unwrap();
                            let mut event_op = EventOp::Ready;
                            mem::swap(&mut event_op, &mut event.writable_op);
                            if let EventOp::Pending(waker) = event_op {
                                wakers.push(waker);
                            }
                        }
                        if (kev.filter == libc::EVFILT_SIGNAL) {}
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
