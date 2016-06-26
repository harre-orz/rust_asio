use std::io;
use std::mem;
use std::ptr;
use std::cell::UnsafeCell;
use std::sync::Mutex;
use std::collections::HashSet;
use {IoObject, IoService};
use super::{Handler, HandlerResult, TaskExecutor};
use ops::*;

struct EpollObject {
    fd: RawFd,
    intr: bool,
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

    pub fn poll(&self, block: bool, task: &TaskExecutor) -> usize {
        let mut events: [epoll_event; 128] = unsafe { mem::uninitialized() };
        let n = epoll_wait(self.epoll_fd, &mut events, if block { -1 } else { 0 });
        for ev in &events[..n] {
            let ptr = unsafe { &mut *(ev.u64 as *mut EpollObject) };
            if ptr.intr {
                if (ev.events & EPOLLIN as u32) != 0 {
                    let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
                    read(ptr, &mut buf).unwrap();
                }
            } else {
                if (ev.events & EPOLLIN as u32) != 0 {
                    if let Some((id, callback)) = {
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
                        if (ev.events & (EPOLLERR | EPOLLHUP) as u32) != 0 {
                            task.post(id, Box::new(
                                move |io| callback(io, HandlerResult::Errored))
                            );
                        } else {
                            task.post(id, Box::new(
                                move |io| callback(io, HandlerResult::Ready))
                            );
                        }
                    }
                }
                if (ev.events & EPOLLOUT as u32) != 0 {
                    if let Some((id, callback)) = {
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
                        if (ev.events & (EPOLLERR | EPOLLHUP) as u32) != 0 {
                            task.post(id, Box::new(
                                move |io| callback(io, HandlerResult::Errored))
                            );
                        } else {
                            task.post(id, Box::new(
                                move |io| callback(io, HandlerResult::Ready))
                            );
                        }
                    }
                }
            }
        }

        let epoll = self.mutex.lock().unwrap();
        epoll.callback_count
    }

    pub fn drain_all(&self, task: &TaskExecutor) {
        let mut count = 0;
        let mut epoll = self.mutex.lock().unwrap();
        for e in &epoll.registered {
            let ptr = unsafe { &mut * e.0 };
            let mut opt = None;
            mem::swap(&mut ptr.in_op, &mut opt);
            if let Some(callback) = opt {
                task.post(ptr.in_id, Box::new(move |io| callback(io, HandlerResult::Canceled)));
                count += 1;
            }
            let mut opt = None;
            mem::swap(&mut ptr.out_op, &mut opt);
            if let Some(callback) = opt {
                task.post(ptr.in_id, Box::new(move |io| callback(io, HandlerResult::Canceled)));
                count += 1;
            }
        }
        epoll.callback_count -= count;
    }

    fn ctl_add_io(&self, ptr: &mut EpollObject) {
        debug_assert!(!ptr.intr);

        let mut ev = epoll_event {
            events: (EPOLLIN | EPOLLOUT | EPOLLET) as u32,
            u64: (ptr as *const EpollObject) as u64,
        };
        epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, ptr.fd, &mut ev).unwrap();
    }

    fn ctl_add_intr(&self, ptr: &mut EpollObject) {
        debug_assert!(ptr.intr);

        let mut ev = epoll_event {
            events: EPOLLIN as u32,
            u64: (ptr as *const EpollObject) as u64,
        };
        epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, ptr.fd, &mut ev).unwrap();
    }

    fn ctl_del(&self, ptr: &mut EpollObject) {
        let mut ev = epoll_event {
            events: 0,
            u64: (ptr as *const EpollObject) as u64,
        };
        epoll_ctl(self.epoll_fd, EPOLL_CTL_DEL, ptr.fd, &mut ev).unwrap();
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
    io: IoService,
    epoll_ptr: Box<UnsafeCell<EpollObject>>,
}

impl EpollIoActor {
    pub fn new(io: &IoService, fd: RawFd) -> EpollIoActor {
        let res = EpollIoActor {
            io: io.clone(),
            epoll_ptr: Box::new(UnsafeCell::new(EpollObject {
                fd: fd,
                intr: false,
                ..Default::default()
            })),
        };
        let ptr = unsafe { &mut *res.epoll_ptr.get() };
        io.0.epoll.ctl_add_io(ptr);
        io.0.epoll.register(ptr);
        res
    }

    pub fn register(&self) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        self.io.0.epoll.ctl_add_io(ptr);
        self.io.0.epoll.register(ptr);
    }

    pub fn set_in(&self, id: usize, callback: Handler) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        let epoll = &self.io.0.epoll;

        let (old, new) = {
            let mut some = Some(callback);
            let mut none = None;
            let mut epoll = epoll.mutex.lock().unwrap();
            if ptr.in_ready {
                mem::swap(&mut ptr.in_op, &mut none);
                if none.is_some() {
                    epoll.callback_count -= 1;
                }
                (none, some)
            } else {
                mem::swap(&mut ptr.in_op, &mut some);
                if some.is_none() {
                    ptr.in_id = id;
                    epoll.callback_count += 1;
                }
                (some, none)
            }
        };
        if let Some(callback) = old {
            self.io.0.task.post(id, Box::new(move |io| callback(io, HandlerResult::Canceled)))
        }
        if let Some(callback) = new {
            self.io.0.task.post(id, Box::new(move |io| callback(io, HandlerResult::Ready)))
        } else {
            self.io.0.interrupt();
        }
    }

    pub fn unset_in(&self) -> Option<Handler> {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };

        let mut epoll = self.io.0.epoll.mutex.lock().unwrap();
        let mut opt = None;
        mem::swap(&mut ptr.in_op, &mut opt);
        if opt.is_some() {
            epoll.callback_count -= 1;
        }
        opt
    }

    pub fn ready_in(&self, ready: bool) -> bool {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };

        let _epoll = self.io.0.epoll.mutex.lock().unwrap();
        let old = ptr.in_ready;
        ptr.in_ready = ready;
        old
    }

    pub fn set_out(&self, id: usize, callback: Handler) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        let epoll = &self.io.0.epoll;

        let (old, new) = {
            let mut some = Some(callback);
            let mut none = None;
            let mut epoll = epoll.mutex.lock().unwrap();
            if ptr.out_ready {
                mem::swap(&mut ptr.out_op, &mut none);
                if none.is_some() {
                    epoll.callback_count -= 1;
                }
                (none, some)
            } else {
                mem::swap(&mut ptr.out_op, &mut some);
                if some.is_none() {
                    ptr.out_id = id;
                    epoll.callback_count += 1;
                }
                (some, none)
            }
        };
        if let Some(callback) = old {
            self.io.0.task.post(id, Box::new(move |io| callback(io, HandlerResult::Canceled)))
        }
        if let Some(callback) = new {
            self.io.0.task.post(id, Box::new(move |io| callback(io, HandlerResult::Ready)))
        } else {
            self.io.0.interrupt();
        }
    }

    pub fn unset_out(&self) -> Option<Handler> {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };

        let mut opt = None;
        let mut epoll = self.io.0.epoll.mutex.lock().unwrap();
        mem::swap(&mut ptr.out_op, &mut opt);
        if opt.is_some() {
            epoll.callback_count -= 1;
        }
        opt
    }

    pub fn ready_out(&self, ready: bool) -> bool {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };

        let epoll = self.io.0.epoll.mutex.lock().unwrap();
        let old = ptr.out_ready;
        ptr.out_ready = ready;
        old
    }

    pub fn reopen(&self, fd: RawFd) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        let epoll = &self.io.0.epoll;

        epoll.ctl_del(ptr);
        let _ = close(ptr);
        ptr.fd = fd;
        epoll.ctl_add_io(ptr);
    }
}

impl IoObject for EpollIoActor {
    fn io_service(&self) -> &IoService {
        &self.io
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
        let epoll = &self.io.0.epoll;
        epoll.unregister(ptr);
        epoll.ctl_del(ptr);
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
        io.0.epoll.ctl_add_intr(ptr);
    }

    pub fn unset_intr(&self, io: &IoService) {
        let ptr = unsafe { &mut *self.epoll_ptr.get() };
        io.0.epoll.ctl_del(ptr);
    }
}

impl AsRawFd for EpollIntrActor {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { &*self.epoll_ptr.get() }.fd
    }
}

impl Drop for EpollIntrActor {
    fn drop(&mut self) {
        let _ = close(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use {IoService, IoObject,Strand};
    use std::thread;
    use std::sync::Arc;
    use libc;
    use test::Bencher;

    fn make_io_actor(io: &IoService) -> EpollIoActor {
        EpollIoActor::new(io, unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) })
    }

    #[test]
    fn test_epoll_set_unset() {
        let io = IoService::new();
        let ev = Strand::new(&io, make_io_actor(&io));

        ev.unset_in();
        assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_none());
        assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_none());

        ev.unset_out();
        assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_none());
        assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_none());

        ev.set_in(0, Box::new(|_,_| {}));
        assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_some());
        assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_none());

        ev.set_out(0, Box::new(|_,_| {}));
        assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_some());
        assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_some());

        let obj = ev.obj.clone();
        let io = io.clone();
        thread::spawn(move || {
            let ev = Strand { io: &io, obj: obj };
            assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_some());
            assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_some());

            ev.unset_in();
            assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_none());
            assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_some());

            ev.unset_out();
            assert!(unsafe { &*ev.epoll_ptr.get() }.in_op.is_none());
            assert!(unsafe { &*ev.epoll_ptr.get() }.out_op.is_none());
        }).join().unwrap();
    }

    #[bench]
    fn bench_epoll_set_in(b: &mut Bencher) {
        let io = IoService::new();
        let ev = Strand::new(&io, make_io_actor(&io));
        b.iter(|| {
            ev.set_in(0, Box::new(|_,_| {}));
        });
    }

    #[bench]
    fn bench_epoll_set_in_unset(b: &mut Bencher) {
        let io = IoService::new();
        let ev = Strand::new(&io, make_io_actor(&io));
        b.iter(|| {
            ev.set_in(0, Box::new(|_,_| {}));
            ev.unset_in();
        });
    }

    #[bench]
    fn bench_epoll_set_out(b: &mut Bencher) {
        let io = IoService::new();
        let ev = Strand::new(&io, make_io_actor(&io));
        b.iter(|| {
            ev.set_out(0, Box::new(|_,_| {}));
        });
    }

    #[bench]
    fn bench_epoll_set_out_unset(b: &mut Bencher) {
        let io = IoService::new();
        let ev = Strand::new(&io, make_io_actor(&io));
        b.iter(|| {
            ev.set_out(0, Box::new(|_,_| {}));
            ev.unset_out()
        });
    }
}
