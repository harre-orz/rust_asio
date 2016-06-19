use std::io;
use std::mem;
use std::ptr;
use std::cell::UnsafeCell;
use std::sync::Mutex;
use std::time::Duration;
use std::collections::HashSet;
use {IoService};
use super::{Backbone, Handler, HandlerResult, Expiry};
use ops::*;

struct EpollObject {
    fd: RawFd,
    intr: bool,
    epoll: *const EpollReactor,
    in_op: Option<Handler>,
    in_id: usize,
    in_ready: bool,
    out_op: Option<Handler>,
    out_id: usize,
    out_ready: bool,
}

impl Default for EpollObject {
    fn default() -> EpollObject {
        EpollObject {
            fd: 0,
            intr: false,
            epoll: ptr::null(),
            in_op: None,
            in_id: 0,
            in_ready: false,
            out_op: None,
            out_id: 0,
            out_ready: false,
        }
    }
}

impl AsRawFd for EpollObject {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

unsafe impl Send for EpollObject {}

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
struct EpollEntry(*mut EpollObject);

unsafe impl Send for EpollEntry {}

struct EpollManage {
    callback_count: usize,
    registered: HashSet<EpollEntry>,
}

impl EpollManage {
    #[inline]
    fn do_nothing(&self) {
    }
}

#[cfg(test)]
impl Drop for EpollManage {
    fn drop(&mut self) {
        debug_assert!(self.registered.is_empty());
    }
}

pub struct EpollReactor {
    epoll_fd: RawFd,
    mutex: Mutex<EpollManage>,
}

impl EpollReactor {
    pub fn new() -> io::Result<EpollReactor> {
        let epoll_fd = try!(epoll_create());
        Ok(EpollReactor {
            epoll_fd: epoll_fd,
            mutex: Mutex::new(EpollManage {
                callback_count: 0,
                registered: HashSet::new(),
            }),
        })
    }

    pub fn poll(&self, expiry: Expiry, vec: &mut Vec<(usize, Handler)>) -> usize {
        let mut events: [epoll_event; 128] = unsafe { mem::uninitialized() };
        let n = epoll_wait(self.epoll_fd, &mut events, &expiry.wait_duration(Duration::new(5, 0)));
        for ev in &events[..n] {
            let ptr = unsafe { &mut *(ev.u64 as *mut EpollObject) };
            if ptr.intr {
                if (ev.events & EPOLLIN as u32) != 0 {
                    let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
                    let _ = recv(ptr, &mut buf, 0);
                }
            } else {
                if (ev.events & EPOLLIN as u32) != 0 {
                    if let Some(op) = {
                        let mut opt = None;
                        let mut epoll = self.mutex.lock().unwrap();
                        ptr.in_ready = true;
                        mem::swap(&mut ptr.in_op, &mut opt);
                        if let Some(callback) = opt {
                            epoll.callback_count -= 1;
                            Some((ptr.in_id, callback))
                        } else {
                            None
                        }
                    } {
                        vec.push(op);
                    }
                }
                if (ev.events & EPOLLOUT as u32) != 0 {
                    if let Some(op) = {
                        let mut opt = None;
                        let mut epoll = self.mutex.lock().unwrap();
                        ptr.out_ready = true;
                        mem::swap(&mut ptr.out_op, &mut opt);
                        if let Some(callback) = opt {
                            epoll.callback_count -= 1;
                            Some((ptr.out_id, callback))
                        } else {
                            None
                        }
                    } {
                        vec.push(op);
                    }
                }
            }
        }
        self.mutex.lock().unwrap().callback_count
    }

    pub fn drain_all(&self, vec: &mut Vec<(usize, Handler)>) {
        let mut count = 0;
        let mut epoll = self.mutex.lock().unwrap();
        for e in &epoll.registered {
            let ptr = unsafe { &mut * e.0 };
            let mut opt = None;
            mem::swap(&mut ptr.in_op, &mut opt);
            if let Some(callback) = opt {
                vec.push((ptr.in_id, callback));
                count += 1;
            }
            let mut opt = None;
            mem::swap(&mut ptr.out_op, &mut opt);
            if let Some(callback) = opt {
                vec.push((ptr.out_id, callback));
                count += 1;
            }
        }
        epoll.callback_count -= count;
    }

    fn ctl_add_io(&self, ptr: &mut EpollObject) {
        debug_assert!(!ptr.intr);
        debug_assert!(ptr.epoll.is_null());

        let mut ev = epoll_event {
            events: (EPOLLIN | EPOLLOUT | EPOLLET) as u32,
            u64: (ptr as *const EpollObject) as u64,
        };
        if let Err(err) = epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, ptr.fd, &mut ev) {
            panic!(err);
        }

        ptr.epoll = self;
    }

    fn ctl_add_intr(&self, ptr: &mut EpollObject) {
        debug_assert!(ptr.intr);
        debug_assert!(ptr.epoll.is_null());

        let mut ev = epoll_event {
            events: EPOLLIN as u32,
            u64: (ptr as *const EpollObject) as u64,
        };
        if let Err(err) = epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, ptr.fd, &mut ev) {
            panic!(err);
        }
        ptr.epoll = self;
    }

    fn ctl_del(&self, ptr: &mut EpollObject) {
        debug_assert!(!ptr.epoll.is_null());

        let mut ev = epoll_event {
            events: 0,
            u64: (ptr as *const EpollObject) as u64,
        };
        if let Err(err) = epoll_ctl(self.epoll_fd, EPOLL_CTL_DEL, ptr.fd, &mut ev) {
            panic!(err);
        }
        ptr.epoll = ptr::null();
    }

    fn register(&self, ptr: *mut EpollObject) {
        let mut epoll = self.mutex.lock().unwrap();
        let e = EpollEntry(ptr);
        debug_assert!(!epoll.registered.contains(&e));
        epoll.registered.insert(e);
    }

    fn unregister(&self, ptr: *mut EpollObject) {
        let mut epoll = self.mutex.lock().unwrap();
        let e = EpollEntry(ptr);
        debug_assert!(epoll.registered.contains(&e));
        epoll.registered.remove(&e);
    }
}

impl AsRawFd for EpollReactor {
    fn as_raw_fd(&self) -> RawFd {
        self.epoll_fd
    }
}

impl Drop for EpollReactor {
    fn drop(&mut self) {
        let _ = close(self);
    }
}

pub struct EpollIoActor {
    epoll_ptr: UnsafeCell<EpollObject>,
}

impl EpollIoActor {
    pub fn new(fd: RawFd) -> EpollIoActor {
        EpollIoActor {
            epoll_ptr: UnsafeCell::new(EpollObject {
                fd: fd,
                intr: false,
                ..Default::default()
            })
        }
    }

    pub fn set_in(&self, io: &IoService, id: usize, callback: Handler) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        let epoll = &io.0.epoll;
        if ptr.epoll.is_null() {
            epoll.ctl_add_io(ptr);
            epoll.register(ptr);
        } else if ptr.epoll != epoll {
            panic!("");
        }

        let mut epoll = epoll.mutex.lock().unwrap();
        if ptr.in_ready {
            let mut opt = None;
            mem::swap(&mut ptr.in_op, &mut opt);
            if let Some(callback) = opt {
                io.0.task.post(ptr.in_id, Box::new(move || {
                    callback(HandlerResult::Canceled);
                }));
                epoll.callback_count -= 1;
            }
            io.0.task.post(id, Box::new(move || {
                callback(HandlerResult::Ready);
            }));
        } else {
            let mut opt = Some(callback);
            mem::swap(&mut ptr.in_op, &mut opt);
            if let Some(callback) = opt {
                io.0.task.post(ptr.in_id, Box::new(move || {
                    callback(HandlerResult::Canceled);
                }));
            } else {
                epoll.callback_count += 1;
            }
            ptr.in_id = id;
        }
        Backbone::interrupt(io);
    }

    pub fn unset_in(&self, io: &IoService)  {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        if ptr.epoll.is_null() {
            return;
        }

        let mut epoll = io.0.epoll.mutex.lock().unwrap();
        let mut opt = None;
        mem::swap(&mut ptr.in_op, &mut opt);
        if let Some(callback) = opt {
            io.0.task.post(ptr.in_id, Box::new(move || {
                callback(HandlerResult::Canceled);
            }));
            epoll.callback_count -= 1;
        }
    }

    pub fn ready_in(&self, io: &IoService, ready: bool) -> bool {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        if ptr.epoll.is_null() {
            return false;
        }

        let epoll = io.0.epoll.mutex.lock().unwrap();
        epoll.do_nothing();
        let old = ptr.in_ready;
        ptr.in_ready = ready;
        old
    }

    pub fn set_out(&self, io: &IoService, id: usize, callback: Handler) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        let epoll = &io.0.epoll;
        if ptr.epoll.is_null() {
            epoll.ctl_add_io(ptr);
            epoll.register(ptr);
        } else if ptr.epoll != epoll {
            panic!("");
        }

        let mut epoll = epoll.mutex.lock().unwrap();
        epoll.do_nothing();
        if ptr.out_ready {
            let mut opt = None;
            mem::swap(&mut ptr.out_op, &mut opt);
            if let Some(callback) = opt {
                io.0.task.post(ptr.out_id, Box::new(move || {
                    callback(HandlerResult::Canceled);
                }));
                epoll.callback_count -= 1;
            }
            io.0.task.post(id, Box::new(move || {
                callback(HandlerResult::Ready)
            }));
        } else {
            let mut opt = Some(callback);
            mem::swap(&mut ptr.out_op, &mut opt);
            if let Some(callback) = opt {
                io.0.task.post(ptr.out_id, Box::new(move || {
                    callback(HandlerResult::Canceled);
                }));
            } else {
                epoll.callback_count += 1;
            }
            ptr.out_id = id;
        }
        Backbone::interrupt(io);
    }

    pub fn unset_out(&self, io: &IoService) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        if ptr.epoll.is_null() {
            return;
        }

        let mut epoll = io.0.epoll.mutex.lock().unwrap();
        let mut opt = None;
        mem::swap(&mut ptr.out_op, &mut opt);
        if let Some(callback) = opt {
            io.0.task.post(ptr.out_id, Box::new(move || {
                callback(HandlerResult::Canceled);
            }));
            epoll.callback_count -= 1;
        }
    }

    pub fn ready_out(&self, io: &IoService, ready: bool) -> bool {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        if ptr.epoll.is_null() {
            return false;
        }

        let epoll = io.0.epoll.mutex.lock().unwrap();
        epoll.do_nothing();
        let old = ptr.out_ready;
        ptr.out_ready = ready;
        old
    }

    pub fn reopen(&self, fd: RawFd) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        debug_assert!(!ptr.epoll.is_null());

        let epoll = unsafe { &*ptr.epoll };
        epoll.ctl_del(ptr);
        let _ = close(ptr);
        ptr.fd = fd;
        epoll.ctl_add_io(ptr);
    }
}

impl AsRawFd for EpollIoActor {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { &*self.epoll_ptr.get() }.fd
    }
}

impl Drop for EpollIoActor {
    fn drop(&mut self) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        if !ptr.epoll.is_null() {
            let epoll = unsafe { &*ptr.epoll };
            epoll.unregister(ptr);
            epoll.ctl_del(ptr);
        }
        let _ = close(self);
    }
}

pub struct EpollIntrActor {
    epoll_ptr: UnsafeCell<EpollObject>,
}

impl EpollIntrActor {
    pub fn new(fd: RawFd) -> EpollIntrActor {
        EpollIntrActor {
            epoll_ptr: UnsafeCell::new(EpollObject {
                fd: fd,
                intr: true,
                ..Default::default()
            })
        }
    }

    pub fn set_intr(&self, io: &IoService) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        if ptr.epoll.is_null() {
            io.0.epoll.ctl_add_intr(ptr);
        }
    }

    pub fn unset_intr(&self, io: &IoService) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        if !ptr.epoll.is_null() {
            io.0.epoll.ctl_del(ptr);
        }
    }
}

impl AsRawFd for EpollIntrActor {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { &*self.epoll_ptr.get() }.fd
    }
}

impl Drop for EpollIntrActor {
    fn drop(&mut self) {
        debug_assert!(unsafe { &*self.epoll_ptr.get() }.epoll.is_null());
        let _ = close(self);
    }
}

#[test]
fn test_epoll_set_unset() {
    use std::thread;
    use {IoService, Strand};
    use libc;

    let io = IoService::new();
    let ev = Strand::new(&io, EpollIoActor::new(unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) }));

    ev.unset_in(&io);
    assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_none());
    assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_none());

    ev.unset_out(&io);
    assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_none());
    assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_none());

    ev.set_in(&io, 0, Box::new(move |_| {}));
    assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_some());
    assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_none());

    ev.set_out(&io, 0, Box::new(move |_| {}));
    assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_some());
    assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_some());

    let arc = ev.0.clone();
    thread::spawn(move || {
        let ev = Strand(arc);
        assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_some());
        assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_some());

        ev.unset_in(&io);
        assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_none());
        assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_some());

        ev.unset_out(&io);
        assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_none());
        assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_none());
    }).join().unwrap();
}
