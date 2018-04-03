use ffi::{AsRawFd, RawFd, close, Signal, SystemError, OPERATION_CANCELED, sock_error};
use core::{IoContext, AsIoContext, ThreadIoContext, TimerQueue, Perform, Expiry, Intr, UnsafeRef};

use std::mem;
use std::ptr;
use std::time::Duration;
use std::sync::Mutex;
use std::collections::{HashSet, VecDeque};
use libc::{self, EV_ADD, EV_ERROR, EV_DELETE, EV_ENABLE, EV_DISPATCH, EV_CLEAR, EVFILT_READ,
           EVFILT_WRITE, EVFILT_SIGNAL, SIG_SETMASK, sigaddset, sigprocmask, sigset_t, sigemptyset};

fn ev_set(kev: &Kevent, ident: i32, filter: i16, flags: u16) -> libc::kevent {
    libc::kevent {
        ident: ident as usize,
        filter: filter,
        flags: flags,
        fflags: 0,
        data: 0,
        udata: kev as *const _ as *mut _,
    }
}

fn dispatch_socket(kev: &libc::kevent, this: &mut ThreadIoContext) {
    let udata = unsafe { &mut *(kev.udata as *const Kevent as *mut Kevent) };
    match kev.filter {
        _ if (kev.flags & EV_ERROR) != 0 => {
            let err = sock_error(udata);
            this.as_ctx().clone().as_reactor().cancel_ops_nolock(
                udata,
                this.as_ctx(),
                err,
            )
        }
        EVFILT_READ => {
            if let Some(op) = udata.input.queue.pop_front() {
                udata.input.blocked = true;
                this.push(op, SystemError::default());
            }
        }
        EVFILT_WRITE => {
            if let Some(op) = udata.output.queue.pop_front() {
                udata.output.blocked = true;
                this.push(op, SystemError::default());
            }
        }
        EVFILT_SIGNAL => {
            if let Some(op) = udata.input.queue.pop_front() {
                let sig: Signal = unsafe { mem::transmute(kev.ident as i32) };
                this.push(op, SystemError::from_signal(sig));
            }
        }
        _ => unreachable!(),
    }
}

fn dispatch_intr(kev: &libc::kevent, _: &mut ThreadIoContext) {
    match kev.filter {
        EVFILT_READ => unsafe {
            let mut buf: [u8; 8] = mem::uninitialized();
            libc::read(kev.ident as RawFd, buf.as_mut_ptr() as *mut _, buf.len());
        },
        _ => unreachable!(""),
    }
}

#[derive(Default)]
struct Ops {
    queue: VecDeque<Box<Perform>>,
    blocked: bool,
    canceled: bool,
}

pub struct Kevent {
    fd: RawFd,
    input: Ops,
    output: Ops,
    dispatch: fn(&libc::kevent, &mut ThreadIoContext),
}

impl Kevent {
    pub fn socket(fd: RawFd) -> Self {
        Kevent {
            fd: fd,
            input: Default::default(),
            output: Default::default(),
            dispatch: dispatch_socket,
        }
    }

    pub fn signal() -> Self {
        Kevent {
            fd: -1,
            input: Ops {
                queue: Default::default(),
                blocked: true, // Always blocked
                canceled: false,
            },
            output: Default::default(),
            dispatch: dispatch_socket,
        }
    }

    pub fn intr(fd: RawFd) -> Self {
        Kevent {
            fd: fd,
            input: Default::default(),
            output: Default::default(),
            dispatch: dispatch_intr,
        }
    }
}

unsafe impl Send for Kevent {}

impl AsRawFd for Kevent {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl PartialEq for Kevent {
    fn eq(&self, other: &Kevent) -> bool {
        (self as *const Self) == (other as *const Self)
    }
}

impl Eq for Kevent {}

type KeventRef = UnsafeRef<Kevent>;

pub struct KqueueReactor {
    kq: RawFd,
    mutex: Mutex<HashSet<KeventRef>>,
    intr: Intr,
    sigmask: Mutex<sigset_t>,
}

impl KqueueReactor {
    pub fn new() -> Result<Self, SystemError> {
        match unsafe { libc::kqueue() } {
            -1 => Err(SystemError::last_error()),
            kq => {
                let kq = KqueueReactor {
                    kq: kq,
                    mutex: Default::default(),
                    intr: Intr::new()?,
                    sigmask: unsafe {
                        let mut sigmask = mem::uninitialized();
                        sigemptyset(&mut sigmask);
                        Mutex::new(sigmask)
                    },
                };
                Ok(kq)
            }
        }
    }

    pub fn init(&self) {
        self.intr.startup(self);
    }

    pub fn kevent(&self, kev: &[libc::kevent]) {
        unsafe {
            libc::kevent(
                self.kq,
                kev.as_ptr(),
                kev.len() as i32,
                ptr::null_mut(),
                0,
                ptr::null(),
            )
        };
    }

    pub fn poll(&self, block: bool, tq: &TimerQueue, this: &mut ThreadIoContext) {
        let tv = if block {
            let timeout = tq.wait_duration(10 * 1_000_000_000);
            let sec = timeout / 1_000_000_000;
            libc::timespec {
                tv_sec: sec as i64,
                tv_nsec: (timeout - (sec * 1_000_000_000)) as i64,
            }
        } else {
            libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }
        };

        let mut kev: [libc::kevent; 128] = unsafe { mem::uninitialized() };
        let n = unsafe {
            libc::kevent(
                self.kq,
                ptr::null(),
                0,
                kev.as_mut_ptr(),
                kev.len() as _,
                &tv,
            )
        };

        tq.get_ready_timers(this);
        if n > 0 {
            let _kq = self.mutex.lock().unwrap();
            for ev in &kev[..(n as usize)] {
                let kev = unsafe { &*(ev.udata as *const Kevent) };
                (kev.dispatch)(ev, this);
            }
        }
    }

    pub fn register_socket(&self, kev: &Kevent) {
        self.kevent(
            &[
                ev_set(
                    kev,
                    kev.fd,
                    EVFILT_READ,
                    EV_ADD | EV_CLEAR | EV_ENABLE | EV_DISPATCH,
                ),
                ev_set(
                    kev,
                    kev.fd,
                    EVFILT_WRITE,
                    EV_ADD | EV_CLEAR | EV_ENABLE | EV_DISPATCH,
                ),
            ],
        );
        let mut kq = self.mutex.lock().unwrap();
        kq.insert(KeventRef::new(kev));
    }

    pub fn deregister_socket(&self, kev: &Kevent) {
        self.kevent(
            &[
                ev_set(kev, kev.fd, EVFILT_READ, EV_DELETE),
                ev_set(kev, kev.fd, EVFILT_WRITE, EV_DELETE),
            ],
        );
        let mut kq = self.mutex.lock().unwrap();
        kq.remove(&KeventRef::new(kev));
    }

    pub fn register_signal(&self, kev: &Kevent) {
        let mut kq = self.mutex.lock().unwrap();
        kq.insert(KeventRef::new(kev));
    }

    pub fn deregister_signal(&self, kev: &Kevent) {
        let mut kq = self.mutex.lock().unwrap();
        kq.remove(&KeventRef::new(kev));
    }

    pub fn register_intr(&self, kev: &Kevent) {
        self.kevent(&[ev_set(kev, kev.fd, EVFILT_READ, EV_ADD | EV_CLEAR)]);
    }

    pub fn deregister_intr(&self, kev: &Kevent) {
        self.kevent(&[ev_set(kev, kev.fd, EVFILT_READ, EV_DELETE | EV_CLEAR)]);
    }

    pub fn interrupt(&self) {
        self.intr.interrupt()
    }

    pub fn reset_timeout(&self, _: Expiry) {
        self.intr.interrupt()
    }

    pub fn add_read_op(
        &self,
        kev: &Kevent,
        this: &mut ThreadIoContext,
        op: Box<Perform>,
        err: SystemError,
    ) {
        let ops = &mut KeventRef::new(kev).input;
        let _kq = self.mutex.lock().unwrap();
        if err == SystemError::default() {
            if ops.queue.is_empty() && !ops.blocked {
                ops.blocked = true;
                this.push(op, SystemError::default());
            } else {
                ops.queue.push_back(op);
            }
        } else if ops.canceled {
            ops.queue.push_front(op);
            for op in ops.queue.drain(..) {
                this.push(op, OPERATION_CANCELED);
            }
            this.as_ctx().as_reactor().kevent(
                &[
                    ev_set(
                        kev,
                        kev.fd,
                        EVFILT_READ,
                        EV_ENABLE,
                    ),
                ],
            );
        } else {
            ops.blocked = false;
            ops.queue.push_front(op);
            this.as_ctx().as_reactor().kevent(
                &[
                    ev_set(
                        kev,
                        kev.fd,
                        EVFILT_READ,
                        EV_ENABLE,
                    ),
                ],
            );
        }
    }

    pub fn add_write_op(
        &self,
        kev: &Kevent,
        this: &mut ThreadIoContext,
        op: Box<Perform>,
        err: SystemError,
    ) {
        let ops = &mut KeventRef::new(kev).output;
        let _kq = self.mutex.lock().unwrap();
        if err == SystemError::default() {
            if ops.queue.is_empty() && !ops.blocked {
                ops.blocked = true;
                this.push(op, SystemError::default());
            } else {
                ops.queue.push_back(op);
            }
        } else if ops.canceled {
            ops.queue.push_front(op);
            for op in ops.queue.drain(..) {
                this.push(op, OPERATION_CANCELED);
            }
            this.as_ctx().as_reactor().kevent(
                &[
                    ev_set(
                        kev,
                        kev.fd,
                        EVFILT_WRITE,
                        EV_ENABLE,
                    ),
                ],
            );
        } else {
            ops.blocked = false;
            ops.queue.push_front(op);
            this.as_ctx().as_reactor().kevent(
                &[
                    ev_set(
                        kev,
                        kev.fd,
                        EVFILT_WRITE,
                        EV_ENABLE,
                    ),
                ],
            );
        }
    }

    pub fn next_read_op(&self, kev: &Kevent, this: &mut ThreadIoContext) {
        let ops = &mut KeventRef::new(kev).input;
        let _kq = self.mutex.lock().unwrap();
        if ops.canceled {
            ops.canceled = false;
            for op in ops.queue.drain(..) {
                this.push(op, OPERATION_CANCELED);
            }
        } else {
            if let Some(op) = ops.queue.pop_front() {
                this.push(op, SystemError::default());
            } else {
                ops.blocked = false;
                this.as_ctx().as_reactor().kevent(
                    &[
                        ev_set(
                            kev,
                            kev.fd,
                            EVFILT_READ,
                            EV_ENABLE,
                        ),
                    ],
                );
            }
        }
    }

    pub fn next_write_op(&self, kev: &Kevent, this: &mut ThreadIoContext) {
        let ops = &mut KeventRef::new(kev).output;
        let _kq = self.mutex.lock().unwrap();
        if ops.canceled {
            ops.canceled = false;
            for op in ops.queue.drain(..) {
                this.push(op, OPERATION_CANCELED);
            }
        } else {
            if let Some(op) = ops.queue.pop_front() {
                this.push(op, SystemError::default());
            } else {
                ops.blocked = false;
                this.as_ctx().as_reactor().kevent(
                    &[
                        ev_set(
                            kev,
                            kev.as_raw_fd(),
                            EVFILT_READ,
                            EV_ENABLE,
                        ),
                    ],
                );
            }
        }
    }

    pub fn cancel_ops(&self, kev: &Kevent, ctx: &IoContext, err: SystemError) {
        let _kq = self.mutex.lock().unwrap();
        self.cancel_ops_nolock(kev, ctx, err)
    }

    pub fn cancel_ops_nolock(&self, kev: &Kevent, ctx: &IoContext, err: SystemError) {
        for ops in &mut [
            &mut KeventRef::new(kev).input,
            &mut KeventRef::new(kev).output,
        ]
        {
            if !ops.canceled {
                ops.canceled = true;
                if !ops.blocked {
                    for op in ops.queue.drain(..) {
                        ctx.do_post((op, err))
                    }
                }
            }
        }
    }

    pub fn add_signal(&self, kev: &Kevent, sig: Signal) {
        unsafe {
            let mut sigmask = self.sigmask.lock().unwrap();
            sigaddset(&mut *sigmask, sig as i32);
            sigprocmask(SIG_SETMASK, &mut *sigmask, ptr::null_mut());
        }
        self.kevent(
            &[ev_set(kev, sig as i32, EVFILT_SIGNAL, EV_ADD | EV_ENABLE)],
        );
    }

    pub fn del_signal(&self, kev: &Kevent, sig: Signal) {
        self.kevent(&[ev_set(kev, sig as i32, EVFILT_SIGNAL, EV_DELETE)]);
    }
}

impl Drop for KqueueReactor {
    fn drop(&mut self) {
        self.intr.cleanup(self);
        close(self.kq);
    }
}
