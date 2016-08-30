use std::io;
use std::cmp;
use std::mem;
use std::sync::Mutex;
use {IoObject, IoService};
use super::{RawFd, AsRawFd, ErrorCode, READY, CANCELED, Handler, close, get_socket_error};
use libc::{EPOLLIN, EPOLLOUT, EPOLLERR, EPOLLHUP, EPOLLET,
           EPOLL_CLOEXEC, EPOLL_CTL_ADD, EPOLL_CTL_DEL, //EPOLL_CTL_MOD,
           c_void, epoll_event, epoll_create1, epoll_ctl, epoll_wait, read};

#[derive(PartialEq)]
enum AsyncMode {
    Nothing,
    Running,
    Canceling,
}

struct EpollOp {
    operation: Option<Handler>,
    ready: bool,
    mode: AsyncMode,
}

struct EpollEntry {
    fd: RawFd,
    intr: bool,
    input: EpollOp,
    output: EpollOp,
}

impl AsRawFd for EpollEntry {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for EpollEntry {
    fn drop(&mut self) {
        close(self)
    }
}

struct EpollData {
    count: usize,
    registered: Vec<*mut EpollEntry>,
}

unsafe impl Send for EpollData {}

unsafe impl Sync for EpollData {}

pub struct EpollReactor {
    epoll_fd: RawFd,
    mutex: Mutex<EpollData>
}

impl EpollReactor {
    pub fn new() -> io::Result<EpollReactor> {
        let epoll_fd = libc_try!(epoll_create1(EPOLL_CLOEXEC));
        Ok(EpollReactor {
            epoll_fd: epoll_fd,
            mutex: Mutex::new(EpollData {
                count: 0,
                registered: Vec::new(),
            })
        })
    }

    pub fn poll(&self, block: bool, io: &IoService) -> usize {
        let mut events: [epoll_event; 128] = unsafe { mem::uninitialized() };
        let len = {
            cmp::max(0, unsafe {
                epoll_wait(self.epoll_fd, events.as_mut_ptr(), events.len() as i32, if block { -1 } else { 0 })
            }) as usize
        };

        let mut count = 0;
        for ev in &events[..len] {
            let e = unsafe { &mut *(ev.u64 as *mut EpollEntry) };
            if e.intr {
                if (ev.events & EPOLLIN as u32) != 0 {
                    unsafe {
                        let mut buf: [u8; 8] = mem::uninitialized();
                        read(e.fd, buf.as_mut_ptr() as *mut c_void, buf.len());
                    };
                }
            } else {
                if (ev.events & (EPOLLERR | EPOLLHUP) as u32) != 0 {
                    let ec = ErrorCode(get_socket_error(e));
                    if let Some(handler) = {
                        let _epoll = self.mutex.lock().unwrap();
                        e.input.operation.take()
                    } {
                        count += 1;
                        io.post(move |io| handler(io, ec));
                    }
                    if let Some(handler) = {
                        let _epoll = self.mutex.lock().unwrap();
                        e.output.operation.take()
                    } {
                        count += 1;
                        io.post(move |io| handler(io, ec));
                    }
                } else {
                    if (ev.events & EPOLLIN as u32) != 0 {
                        if let Some(handler) = {
                            let _epoll = self.mutex.lock().unwrap();
                            if e.input.operation.is_none() {
                                e.input.ready = true;
                            }
                            e.input.operation.take()
                        } {
                            count += 1;
                            io.post(move |io| handler(io, ErrorCode(READY)));
                        }
                    }
                    if (ev.events & EPOLLOUT as u32) != 0 {
                        if let Some(handler) = {
                            let _epoll = self.mutex.lock().unwrap();
                            if e.output.operation.is_none() {
                                e.output.ready = true;
                            }
                            e.output.operation.take()
                        } {
                            count += 1;
                            io.post(move |io| handler(io, ErrorCode(READY)));
                        }
                    }
                }
            }
        }

        let mut epoll = self.mutex.lock().unwrap();
        epoll.count -= count;
        epoll.count
    }

    pub fn cancel_all(&self, io: &IoService) {
        let mut count = 0;
        let mut epoll = self.mutex.lock().unwrap();
        for e in &epoll.registered {
            let e = unsafe { &mut **e };
            if let Some(handler) = e.input.operation.take() {
                io.post(move |io| handler(io, ErrorCode(CANCELED)));
                count += 1;
            }
            if let Some(handler) = e.output.operation.take() {
                io.post(move |io| handler(io, ErrorCode(CANCELED)));
                count += 1;
            }
        }
        epoll.count -= count;
    }

    fn register(&self, e: *mut EpollEntry)  {
        let mut epoll = self.mutex.lock().unwrap();
        epoll.registered.push(e);
    }

    fn unregister(&self, e: *mut EpollEntry) {
        let fd = unsafe { &*e }.fd;
        let mut epoll = self.mutex.lock().unwrap();
        let idx = epoll.registered.iter().position(|e| unsafe { &**e }.fd == fd).unwrap();
        epoll.registered.remove(idx);
    }

    fn ctl_add_io(&self, e: *const EpollEntry) -> io::Result<()> {
        let mut ev = epoll_event {
            events: (EPOLLIN | EPOLLOUT | EPOLLET) as u32,
            u64: e as u64,
        };
        let e = unsafe { &* e};
        libc_try!(epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, e.fd, &mut ev));
        Ok(())
    }

    fn ctl_add_intr(&self, e: *const EpollEntry) -> io::Result<()> {
        let mut ev = epoll_event {
            events: EPOLLIN as u32,
            u64: e as u64,
        };
        let e = unsafe { &* e};
        libc_try!(epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, e.fd, &mut ev));
        Ok(())
    }

    fn ctl_del(&self, e: *const EpollEntry) -> io::Result<()> {
        let mut ev = epoll_event {
            events: 0,
            u64: 0,
        };
        let e = unsafe { &* e};
        libc_try!(epoll_ctl(self.epoll_fd, EPOLL_CTL_DEL, e.fd, &mut ev));
        Ok(())
    }
}

impl AsRawFd for EpollReactor {
    fn as_raw_fd(&self) -> RawFd {
        self.epoll_fd
    }
}

impl Drop for EpollReactor {
    fn drop(&mut self) {
        close(self)
    }
}


pub struct EpollIoActor {
    io: IoService,
    epoll_ptr: *mut EpollEntry,
}

impl EpollIoActor {
    pub fn new<T: IoObject>(io: &T, fd: RawFd) -> EpollIoActor {
        let io = io.io_service().clone();
        let epoll_ptr = Box::into_raw(Box::new(EpollEntry {
            fd: fd,
            intr: false,
            input: EpollOp {
                operation: None,
                ready: false,
                mode: AsyncMode::Nothing,
            },
            output: EpollOp {
                operation: None,
                ready: false,
                mode: AsyncMode::Nothing,
            },
        }));
        io.0.react.register(epoll_ptr);
        io.0.react.ctl_add_io(epoll_ptr).unwrap();
        EpollIoActor {
            io: io,
            epoll_ptr: epoll_ptr,
        }
    }

    fn set(io: &IoService, op: &mut EpollOp, handler: Handler) {
        let (old, new, canceled) = {
            let mut epoll = io.0.react.mutex.lock().unwrap();
            let canceled = if op.mode == AsyncMode::Canceling {
                true
            } else {
                op.mode = AsyncMode::Running;
                false
            };
            let opt = op.operation.take();
            if op.ready {
                op.ready = false;
                if opt.is_some() {
                    epoll.count -= 1;
                }
                (opt, Some(handler), canceled)
            } else if canceled {
                (opt, Some(handler), true)
            } else {
                op.operation = Some(handler);
                if opt.is_none() {
                    epoll.count += 1;
                }
                (opt, None, false)
            }
        };

        if let Some(handler) = old {
            io.post(|io| handler(io, ErrorCode(CANCELED)));
        }
        if let Some(handler) = new {
            if canceled {
                io.post(|io| handler(io, ErrorCode(CANCELED)));
            } else {
                io.post(|io| handler(io, ErrorCode(READY)));
            }
        }
    }

    pub fn set_input(&self, handler: Handler) {
        let e = unsafe { &mut *self.epoll_ptr };
        Self::set(&self.io, &mut e.input, handler)
    }

    pub fn set_output(&self, handler: Handler) {
        let e = unsafe { &mut *self.epoll_ptr };
        Self::set(&self.io, &mut e.output, handler)
    }

    fn unset(io: &IoService, op: &mut EpollOp) -> Option<Handler> {
        let mut epoll = io.0.react.mutex.lock().unwrap();
        let opt = op.operation.take();
        if opt.is_some() {
            epoll.count -= 1;
        } else if op.mode == AsyncMode::Running {
            op.mode = AsyncMode::Canceling;
        }
        opt
    }

    pub fn unset_input(&self) -> Option<Handler> {
        let e = unsafe { &mut *self.epoll_ptr };
        Self::unset(&self.io, &mut e.input)
    }

    pub fn unset_output(&self) -> Option<Handler> {
        let e = unsafe { &mut *self.epoll_ptr };
        Self::unset(&self.io, &mut e.output)
    }

    fn ready(io: &IoService, op: &mut EpollOp) {
        let _epoll = io.0.react.mutex.lock().unwrap();
        op.ready = true;
        op.mode = AsyncMode::Nothing;
    }

    pub fn ready_input(&self) {
        let e = unsafe { &mut *self.epoll_ptr };
        Self::ready(&self.io, &mut e.input);
    }

    pub fn ready_output(&self) {
        let e = unsafe { &mut *self.epoll_ptr };
        Self::ready(&self.io, &mut e.output);
    }
}

impl IoObject for EpollIoActor {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}

impl AsRawFd for EpollIoActor {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { &*self.epoll_ptr }.fd
    }
}

impl Drop for EpollIoActor {
    fn drop(&mut self) {
        self.io.0.react.ctl_del(self.epoll_ptr).unwrap();
        self.io.0.react.unregister(self.epoll_ptr);
        unsafe { Box::from_raw(self.epoll_ptr) };
    }
}

unsafe impl Send for EpollIoActor {}

unsafe impl Sync for EpollIoActor {}


pub struct EpollIntrActor {
    epoll_ptr: *mut EpollEntry,
}

impl EpollIntrActor {
    pub fn new(fd: RawFd) -> EpollIntrActor {
        EpollIntrActor {
            epoll_ptr: Box::into_raw(Box::new(EpollEntry {
                fd: fd,
                intr: true,
                input: EpollOp {
                    operation: None,
                    ready: false,
                    mode: AsyncMode::Nothing,
                },
                output: EpollOp {
                    operation: None,
                    ready: false,
                    mode: AsyncMode::Nothing,
                },
            }))
        }
    }

    pub fn set_intr(&self, io: &IoService) {
        let data = unsafe { &mut *self.epoll_ptr };
        io.0.react.ctl_add_intr(data).unwrap();
    }

    pub fn unset_intr(&self, io: &IoService) {
        let data = unsafe { &mut *self.epoll_ptr };
        io.0.react.ctl_del(data).unwrap();
    }
}

impl AsRawFd for EpollIntrActor {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { &*self.epoll_ptr }.fd
    }
}

impl Drop for EpollIntrActor {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.epoll_ptr) };
    }
}

unsafe impl Send for EpollIntrActor {}

unsafe impl Sync for EpollIntrActor {}


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use libc::{socket, AF_INET, SOCK_DGRAM};
    use IoService;

    fn make_io_actor(io: &IoService) -> EpollIoActor {
        EpollIoActor::new(io, unsafe { socket(AF_INET, SOCK_DGRAM, 0) })
    }

    fn epoll_count(io: &IoService) -> usize {
        let epoll = io.0.react.mutex.lock().unwrap();
        epoll.count
    }

    #[test]
    fn test_epoll_set_unset() {
        let io = &IoService::new();
        let ev = make_io_actor(io);

        ev.set_input(Box::new(|_, _| {}));
        assert!(unsafe { &*ev.epoll_ptr }.input.operation.is_some());
        assert!(unsafe { &*ev.epoll_ptr }.output.operation.is_none());
        assert_eq!(epoll_count(io), 1);

        ev.set_output(Box::new(|_, _| {}));
        assert!(unsafe { &*ev.epoll_ptr }.input.operation.is_some());
        assert!(unsafe { &*ev.epoll_ptr }.output.operation.is_some());
        assert_eq!(epoll_count(io), 2);

        assert!(ev.unset_input().is_some());
        assert!(unsafe { &*ev.epoll_ptr }.input.operation.is_none());
        assert!(unsafe { &*ev.epoll_ptr }.output.operation.is_some());
        assert_eq!(epoll_count(io), 1);

        assert!(ev.unset_output().is_some());
        assert!(unsafe { &*ev.epoll_ptr }.input.operation.is_none());
        assert!(unsafe { &*ev.epoll_ptr }.output.operation.is_none());
        assert_eq!(epoll_count(io), 0);
    }

    #[bench]
    fn bench_epoll_set(b: &mut Bencher) {
        let io = &IoService::new();
        let ev = make_io_actor(io);
        b.iter(|| ev.set_input(Box::new(|_, _| {})));
    }

    #[bench]
    fn bench_epoll_unset(b: &mut Bencher) {
        let io = &IoService::new();
        let ev = make_io_actor(io);
        b.iter(|| ev.unset_input());
    }

    #[bench]
    fn bench_epoll_set_unset(b: &mut Bencher) {
        let io = &IoService::new();
        let ev = make_io_actor(io);
        b.iter(|| {
            ev.set_input(Box::new(|_, _| {}));
            ev.unset_input();
        });
    }
}
