use std::io;
use std::mem;
use std::cell::UnsafeCell;
use std::sync::{Mutex, Condvar};
use std::collections::HashSet;
use std::time::Duration;
use {IoObject, IoService};
use super::{UseService, Handler, Expiry};
use ops::*;

struct EpollOp {
    callback: Handler,
    id: usize,
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

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
struct EpollEntry(*mut EpollObject);

unsafe impl Send for EpollEntry {}

struct EpollFd {
    fd: RawFd,
    io_count: usize,
    polling: bool,
    actors: HashSet<EpollEntry>,
}

impl AsRawFd for EpollFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for EpollFd {
    fn drop(&mut self) {
        assert!(self.actors.is_empty());
        assert!(self.polling == false);
        let _ = close(self);
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
                actors: HashSet::new(),
            }),
            condvar: Condvar::new(),
        })
    }

    pub fn poll(&self, expiry: Expiry, vec: &mut Vec<(usize, Handler)>) -> usize {
        let epoll_fd = {
            let mut epoll = self.mutex.lock().unwrap();
            while epoll.polling {
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

    fn register(&self, ptr: *mut EpollObject) {
        let mut epoll = self.mutex.lock().unwrap();
        assert!(!epoll.actors.contains(&EpollEntry(ptr)));
        epoll.actors.insert(EpollEntry(ptr));
    }

    fn unregister(&self, ptr: *mut EpollObject) {
        let mut epoll = self.mutex.lock().unwrap();
        assert!(epoll.actors.contains(&EpollEntry(ptr)));
        epoll.actors.remove(&EpollEntry(ptr));
    }

    fn do_event<Tag: EpollTag>(&self, ev: &epoll_event, tag: Tag, vec: &mut Vec<(usize, Handler)>) {
        if Tag::in_event(ev) {
            let ptr: &mut EpollObject = unsafe { mem::transmute(ev.u64) };
            if !ptr.intr {
                if let Some(op) = self.do_reset(ptr, tag, None) {
                    vec.push(op);
                }
            } else {
                let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
                let _ = recv(ptr, &mut buf, 0);
            }
        }
    }

    fn do_reset<Tag: EpollTag>(&self, ptr: &mut EpollObject, _: Tag, mut opt: Option<EpollOp>) -> Option<(usize, Handler)> {
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
            Some((old_op.id, old_op.callback))
        } else {
            None
        }
    }

    fn intr_add(&self, ptr: &EpollObject) {
        let mut ev = epoll_event {
            events: (EPOLLIN) as u32,
            u64: unsafe { mem::transmute(ptr) },
        };

        let epoll = self.mutex.lock().unwrap();
        epoll_ctl(epoll.fd, EPOLL_CTL_ADD, ptr.fd, &mut ev);
    }

    fn intr_del(&self, ptr: &EpollObject) {
        let mut ev = epoll_event {
            events: 0,
            u64: unsafe { mem::transmute(ptr) },
        };

        let epoll = self.mutex.lock().unwrap();
        epoll_ctl(epoll.fd, EPOLL_CTL_DEL, ptr.fd, &mut ev);
    }

    pub fn drain_all(&self, vec: &mut Vec<(usize, Handler)>) {
        let epoll = self.mutex.lock().unwrap();
        for e in &epoll.actors {
            if let Some(op) = self.do_reset(unsafe { &mut *e.0 }, EpollIn, None) {
                vec.push(op);
            }
            if let Some(op) = self.do_reset(unsafe { &mut *e.0 }, EpollOut, None) {
                vec.push(op);
            }
        }
    }
}

pub struct EpollIoActor {
    io: IoService,
    epoll_ptr: Box<UnsafeCell<EpollObject>>,
}

impl EpollIoActor {
    pub fn register(io: &IoService, fd: RawFd) -> EpollIoActor {
        let actor = EpollIoActor {
            io: io.clone(),
            epoll_ptr: Box::new(UnsafeCell::new(EpollObject {
                fd: fd,
                intr: false,
                in_op: None,
                out_op: None,
            })),
        };
        actor.use_service().register(actor.epoll_ptr.get());
        actor
    }

    fn use_service(&self) -> &EpollReactor {
        self.io.use_service()
    }

    pub fn unregister(&self) {
        self.use_service().unregister(self.epoll_ptr.get());
    }

    pub fn set_in(&self, callback: Handler, id: usize) -> Option<Handler> {
        if let Some(op) = self.use_service().do_reset(unsafe { &mut *self.epoll_ptr.get() }, EpollIn, Some(EpollOp { callback: callback, id: id })) {
            self.io.interrupt();
            Some(op.1)
        } else {
            self.io.interrupt();
            None
        }
    }

    pub fn set_out(&self, callback: Handler, id: usize) -> Option<Handler> {
        if let Some(op) = self.use_service().do_reset(unsafe { &mut *self.epoll_ptr.get() }, EpollOut, Some(EpollOp { callback: callback, id: id })) {

            self.io.interrupt();
            Some(op.1)
        } else {

            None
        }
    }

    pub fn unset_in(&self) -> Option<Handler> {
        if let Some(op) = self.use_service().do_reset(unsafe { &mut *self.epoll_ptr.get() }, EpollIn, None) {
            self.io.interrupt();
            Some(op.1)
        } else {
            self.io.interrupt();
            None
        }
    }

    pub fn unset_out(&self) -> Option<Handler> {
        if let Some(op) = self.use_service().do_reset(unsafe { &mut *self.epoll_ptr.get() }, EpollOut, None) {
            self.io.interrupt();
            Some(op.1)
        } else {
            self.io.interrupt();
            None
        }
    }
}

unsafe impl Sync for EpollIoActor {}

impl IoObject for EpollIoActor {
    fn io_service(&self) -> IoService {
        self.io.clone()
    }
}

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
    let ev = EpollIoActor::register(&io, 0);
    assert!(ev.unset_in().is_none());
    assert!(ev.unset_out().is_none());
    assert!(ev.set_in(Box::new(|_| {}), 0).is_none());
    assert!(ev.set_in(Box::new(|_| {}), 0).is_some());
    assert!(ev.set_out(Box::new(|_| {}), 0).is_none());
    assert!(ev.set_out(Box::new(|_| {}), 0).is_some());
    assert!(thread::spawn(move || {
        assert!(ev.unset_in().is_some());
        assert!(ev.unset_out().is_some());
        assert!(ev.unset_in().is_none());
        assert!(ev.unset_out().is_none());
        ev.unregister();
     }).join().is_ok());
}
