use std::io;
use std::mem;
use std::cell::UnsafeCell;
use std::boxed::FnBox;
use std::sync::{Arc, Mutex, Condvar};
use std::time::Duration;
use super::{UseService, Handler, Expiry};
use ops::*;

struct EpollOp {
    callback: Handler,
}

struct EpollObject {
    fd: RawFd,
    intr: bool,
    in_op: Option<EpollOp>,
    out_op: Option<EpollOp>,
}

impl AsRawFd for EpollObject {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

trait EpollTag {
    fn in_event(ev: &epoll_event) -> bool;
    fn swap_op(ptr: &mut EpollObject, op: &mut Option<EpollOp>);
    fn ctrl_mod(ptr: &EpollObject, ev: &mut epoll_event) -> EPOLL_CTL;
}

struct EpollIn;
impl EpollTag for EpollIn {
    fn in_event(ev: &epoll_event) -> bool {
        ev.events & EPOLLIN as u32 != 0
    }

    fn swap_op(ptr: &mut EpollObject, op: &mut Option<EpollOp>) {
        mem::swap(&mut ptr.in_op, op);
    }

    fn ctrl_mod(ptr: &EpollObject, ev: &mut epoll_event) -> EPOLL_CTL {
        match (ptr.in_op.is_some(), ptr.out_op.is_some()) {
            (true, true) => {
                ev.events |= (EPOLLIN | EPOLLOUT) as u32;
                EPOLL_CTL_MOD
            },
            (true, false) => {
                ev.events |= EPOLLIN as u32;
                EPOLL_CTL_ADD
            },
            (false, true) => {
                ev.events |= EPOLLOUT as u32;
                EPOLL_CTL_MOD
            },
            (false, false) => {
                EPOLL_CTL_DEL
            },
        }
    }
}

struct EpollOut;
impl EpollTag for EpollOut {
    fn in_event(ev: &epoll_event) -> bool {
        ev.events & EPOLLOUT as u32 != 0
    }

    fn swap_op(ptr: &mut EpollObject, op: &mut Option<EpollOp>) {
        mem::swap(&mut ptr.out_op, op);
    }

    fn ctrl_mod(ptr: &EpollObject, ev: &mut epoll_event) -> EPOLL_CTL {
        match (ptr.in_op.is_some(), ptr.out_op.is_some()) {
            (true, true) => {
                ev.events |= (EPOLLIN | EPOLLOUT) as u32;
                EPOLL_CTL_MOD
            },
            (true, false) => {
                ev.events |= EPOLLIN as u32;
                EPOLL_CTL_MOD
            },
            (false, true) => {
                ev.events |= EPOLLOUT as u32;
                EPOLL_CTL_ADD
            },
            (false, false) => {
                EPOLL_CTL_DEL
            },
        }
    }
}

struct EpollFd {
    fd: RawFd,
    io_count: usize,
    polling: bool,
}

impl AsRawFd for EpollFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for EpollFd {
    fn drop(&mut self) {
        close(self);
    }
}

pub struct EpollReactor {
    mutex: Mutex<EpollFd>,
    condvar: Condvar,
}

impl EpollReactor {
    pub fn new() -> io::Result<EpollReactor> {
        let epoll_fd = try!(epoll_create());
        Ok(EpollReactor {
            mutex: Mutex::new(EpollFd {
                fd: epoll_fd,
                io_count: 0,
                polling: false,
            }),
            condvar: Condvar::new(),
        })
    }

    pub fn poll(&self, expiry: &Expiry, vec: &mut Vec<Handler>) -> usize {
        let epoll_fd = {
            let mut epoll = self.mutex.lock().unwrap();
            while epoll.polling {
                panic!("poll function dual raced");
                epoll = self.condvar.wait(epoll).unwrap();
            }
            epoll.polling = true;
            epoll.fd
        };
        let mut events: [epoll_event; 128] = unsafe { mem::uninitialized() };
        let timeout = expiry.wait_duration(Duration::new(5, 0));
        let n = epoll_wait(epoll_fd, &mut events, &timeout);
        for ev in &events[..n] {
            self.do_event(&ev, EpollIn, vec);
            self.do_event(&ev, EpollOut, vec);
        }

        let mut epoll = self.mutex.lock().unwrap();
        epoll.polling = false;
        self.condvar.notify_one();
        epoll.io_count
    }

    fn do_event<Tag: EpollTag>(&self, ev: &epoll_event, tag: Tag, vec: &mut Vec<Handler>) {
        if Tag::in_event(ev) {
            let ptr: &mut EpollObject = unsafe { mem::transmute(ev.u64) };
            if !ptr.intr {
                if let Some(callback) = self.do_reset(ptr, tag, None) {
                    vec.push(callback);
                }
            } else {
                let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
                recv(ptr, &mut buf, 0);
            }
        }
    }

    fn do_reset<Tag: EpollTag>(&self, ptr: &mut EpollObject, tag: Tag, mut opt: Option<EpollOp>) -> Option<Handler> {
        let mut ev = epoll_event {
            events: (EPOLLET as u32),
            u64: unsafe { mem::transmute(&*ptr) },
        };

        let mut epoll = self.mutex.lock().unwrap();
        if opt.is_some() {
            epoll.io_count += 1;
        }
        Tag::swap_op(ptr, &mut opt);
        let ptr = &*ptr;

        let op = Tag::ctrl_mod(ptr, &mut ev);
        epoll_ctl(epoll.fd, op, ptr.fd, &mut ev);

        if let Some(old_op) = opt {
            epoll.io_count -= 1;
            Some(old_op.callback)
        } else {
            None
        }
    }

    fn intr_add(&self, ptr: &EpollObject) {
        let mut ev = epoll_event {
            events: (EPOLLIN) as u32,
            u64: unsafe { mem::transmute(ptr) },
        };

        let mut epoll = self.mutex.lock().unwrap();
        epoll_ctl(epoll.fd, EPOLL_CTL_ADD, ptr.fd, &mut ev);
    }

    fn intr_del(&self, ptr: &EpollObject) {
        let mut ev = epoll_event {
            events: 0,
            u64: unsafe { mem::transmute(ptr) },
        };

        let mut epoll = self.mutex.lock().unwrap();
        epoll_ctl(epoll.fd, EPOLL_CTL_DEL, ptr.fd, &mut ev);
    }
}

pub struct EpollIoActor {
    epoll_ptr: Box<UnsafeCell<EpollObject>>,
}

impl EpollIoActor {
    pub fn new(fd: RawFd) -> EpollIoActor {
        EpollIoActor {
            epoll_ptr: Box::new(UnsafeCell::new(EpollObject {
                fd: fd,
                intr: false,
                in_op: None,
                out_op: None,
            })),
        }
    }

    pub fn set_in<T: UseService<EpollReactor>>(&self, io: &T, callback: Handler) -> Option<Handler> {
        io.use_service().do_reset(unsafe { &mut *self.epoll_ptr.get() }, EpollIn, Some(EpollOp { callback: callback }))
    }

    pub fn set_out<T: UseService<EpollReactor>>(&self, io: &T, callback: Handler) -> Option<Handler> {
        io.use_service().do_reset(unsafe { &mut *self.epoll_ptr.get() }, EpollOut, Some(EpollOp { callback: callback }))
    }

    pub fn unset_in<T: UseService<EpollReactor>>(&self, io: &T) -> Option<Handler> {
        io.use_service().do_reset(unsafe { &mut *self.epoll_ptr.get() }, EpollIn, None)
    }

    pub fn unset_out<T: UseService<EpollReactor>>(&self, io: &T) -> Option<Handler> {
        io.use_service().do_reset(unsafe { &mut *self.epoll_ptr.get() }, EpollOut, None)
    }
}

unsafe impl Sync for EpollIoActor {}

impl AsRawFd for EpollIoActor {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { &*self.epoll_ptr.get() }.fd
    }
}

pub struct EpollIntrActor {
    epoll_ptr: Box<UnsafeCell<EpollObject>>,
}

impl EpollIntrActor {
    pub fn new(fd: RawFd) -> EpollIntrActor {
        EpollIntrActor {
            epoll_ptr: Box::new(UnsafeCell::new(EpollObject {
                fd: fd,
                intr: true,
                in_op: None,
                out_op: None,
            }))
        }
    }

    pub fn set_intr<T: UseService<EpollReactor>>(&self, io: &T) {
        io.use_service().intr_add(unsafe { &*self.epoll_ptr.get() })
    }

    pub fn unset_intr<T: UseService<EpollReactor>>(&self, io: &T) {
        io.use_service().intr_del(unsafe { &*self.epoll_ptr.get() })
    }
}

impl AsRawFd for EpollIntrActor {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { &*self.epoll_ptr.get() }.fd
    }
}

#[test]
fn test_epoll_set_unset() {
    use std::thread;
    use IoService;

    let io = IoService::new();
    let ev = EpollIoActor::new(0);
    assert!(ev.unset_in(&io).is_none());
    assert!(ev.unset_out(&io).is_none());
    assert!(ev.set_in(&io, Box::new(|_| {})).is_none());
    assert!(ev.set_in(&io, Box::new(|_| {})).is_some());
    assert!(ev.set_out(&io, Box::new(|_| {})).is_none());
    assert!(ev.set_out(&io, Box::new(|_| {})).is_some());
    thread::spawn(move || {
        assert!(ev.unset_in(&io).is_some());
        assert!(ev.unset_out(&io).is_some());
        assert!(ev.unset_in(&io).is_none());
        assert!(ev.unset_out(&io).is_none());
     }).join();
}
