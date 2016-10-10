use std::mem;
use std::os::unix::io::{RawFd, AsRawFd};
use std::sync::{Mutex};
use std::collections::VecDeque;
use error::{ErrorCode, READY, CANCELED};
use unsafe_cell::UnsafeBoxedCell;
use super::{IoObject, IoService, ThreadInfo, Callback};
use libc::{EPOLLIN, EPOLLOUT, EPOLLERR, EPOLLHUP, EPOLLET,
           EPOLL_CLOEXEC, EPOLL_CTL_ADD, EPOLL_CTL_DEL, //EPOLL_CTL_MOD,
           c_void, epoll_event, epoll_create1, epoll_ctl, epoll_wait, close, read,
           SOL_SOCKET, SO_ERROR, getsockopt, socklen_t};

fn getsockerr(fd: RawFd) -> ErrorCode {
    let mut ec = 0i32;
    let mut len = mem::size_of::<i32>() as socklen_t;
    libc_ign!(getsockopt(fd, SOL_SOCKET, SO_ERROR, &mut ec as *mut _ as *mut c_void, &mut len));
    ErrorCode(ec)
}

#[derive(Default)]
struct Op {
    ops: VecDeque<Callback>,
    ready: bool,
    canceling: bool,
}

struct Entry {
    fd: RawFd,
    intr: bool,
    input: Op,
    output: Op,
}

struct ReactorData {
    callback_count: usize,
    registered_entry: Vec<*mut Entry>,
}

pub struct Reactor {
    epoll_fd: RawFd,
    mutex: Mutex<ReactorData>,
}

unsafe impl Send for Reactor {
}

unsafe impl Sync for Reactor {
}

impl Reactor {
    pub fn new() -> Reactor {
        let epoll_fd = libc_unwrap!(epoll_create1(EPOLL_CLOEXEC));
        Reactor {
            epoll_fd: epoll_fd,
            mutex: Mutex::new(ReactorData {
                callback_count: 0,
                registered_entry: Vec::new(),
            })
        }
    }

    pub fn poll(&self, timeout: Option<i32>, io: &IoService, ti: &ThreadInfo) -> usize {
        let mut events: [epoll_event; 128] = unsafe { mem::uninitialized() };
        let len = unsafe {
            epoll_wait(self.epoll_fd, events.as_mut_ptr(), events.len() as i32, timeout.unwrap_or(0))
        };
        if len > 0 {
            for ev in &events[..(len as usize)] {
                let ptr = unsafe { &mut *(ev.u64 as *mut Entry) };
                if ptr.intr {
                    if (ev.events & EPOLLIN as u32) != 0 {
                        let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
                        libc_ign!(read(ptr.fd, buf.as_mut_ptr() as *mut c_void, buf.len()));
                    }
                } else {
                    if (ev.events & (EPOLLERR | EPOLLHUP) as u32) != 0 {
                        let ec = getsockerr(ptr.fd);

                        let mut epoll = self.mutex.lock().unwrap();
                        while let Some(callback) = ptr.input.ops.pop_front() {
                            epoll.callback_count -= 1;
                            io.post(move |io| callback(io, ec));
                        }
                        while let Some(callback) = ptr.output.ops.pop_front() {
                            epoll.callback_count -= 1;
                            io.post(move |io| callback(io, ec));
                        }
                    } else {
                        if (ev.events & EPOLLIN as u32) != 0 {
                            let mut epoll = self.mutex.lock().unwrap();
                            if let Some(callback) = ptr.input.ops.pop_front() {
                                epoll.callback_count -= 1;
                                ti.push(callback);
                                ptr.input.ready = false;
                            } else {
                                ptr.input.ready = true;
                            }
                        }
                        if (ev.events & EPOLLOUT as u32) != 0 {
                            let mut epoll = self.mutex.lock().unwrap();
                            if let Some(callback) = ptr.output.ops.pop_front() {
                                epoll.callback_count -= 1;
                                ti.push(callback);
                                ptr.output.ready = false;
                            } else {
                                ptr.output.ready = true;
                            }
                        }
                    }
                }
            }
        }

        let epoll = self.mutex.lock().unwrap();
        return epoll.callback_count;
    }

    pub fn cancel_all(&self, ti: &ThreadInfo) {
        let mut epoll = self.mutex.lock().unwrap();
        for ptr in &epoll.registered_entry {
            while let Some(callback) = unsafe { &mut **ptr }.input.ops.pop_front() {
                ti.push(callback);
            }
            while let Some(callback) = unsafe { &mut **ptr }.output.ops.pop_front() {
                ti.push(callback);
            }
        }
        epoll.callback_count = 0;
    }

    fn register(&self, ptr: &mut Entry)  {
        let mut epoll = self.mutex.lock().unwrap();
        epoll.registered_entry.push(ptr);
    }

    fn unregister(&self, ptr: &mut Entry) {
        let mut epoll = self.mutex.lock().unwrap();
        let idx = epoll.registered_entry.iter().position(|&e| unsafe { &*e }.fd == ptr.fd).unwrap();
        epoll.registered_entry.remove(idx);
    }

    fn epoll_ctl(&self, e: &Entry, op: i32, events: i32) {
        let mut ev = epoll_event {
            events: events as u32,
            u64: e as *const _ as u64,
        };
        libc_unwrap!(epoll_ctl(self.epoll_fd, op, e.fd, &mut ev));
    }

    fn add_op(&self, op: &mut Op, callback: Callback, try_again: bool) -> Result<Option<Callback>, Vec<Callback>> {
        let mut epoll = self.mutex.lock().unwrap();
        if op.canceling && try_again {
            epoll.callback_count -= op.ops.len();
            op.ops.push_front(callback);
            Err(op.ops.drain(..).collect())
        } else {
            op.canceling = false;
            if op.ready {
                op.ready = false;
                if op.ops.is_empty() || try_again {
                    Ok(Some(callback))
                } else {
                    op.ops.push_back(callback);
                    Ok(op.ops.pop_front())
                }
            } else {
                op.ready = false;
                epoll.callback_count += 1;
                if try_again {
                    op.ops.push_front(callback);
                } else {
                    op.ops.push_back(callback);
                }
                Ok(None)
            }
        }
    }

    fn next_op(&self, op: &mut Op) -> Option<Result<Callback, Vec<Callback>>> {
        let mut epoll = self.mutex.lock().unwrap();
        if !op.canceling {
            if let Some(callback) = op.ops.pop_front() {
                epoll.callback_count -= 1;
                Some(Ok(callback))
            } else {
                op.ready = true;
                None
            }
        } else {
            op.canceling = false;
            op.ready = true;
            let len = op.ops.len();
            epoll.callback_count -= len;
            if len > 0 {
                Some(Err(op.ops.drain(..).collect()))
            } else {
                None
            }
        }
    }

    fn del_ops(&self, op: &mut Op) -> Vec<Callback> {
        let mut epoll = self.mutex.lock().unwrap();
        let ops: Vec<Callback> = op.ops.drain(..).collect();
        epoll.callback_count -= ops.len();
        op.canceling = true;
        ops
    }
}

impl Drop for Reactor {
    fn drop(&mut self) {
        libc_ign!(close(self.epoll_fd));
    }
}


pub struct IntrActor {
    ptr: UnsafeBoxedCell<Entry>,
}

impl IntrActor {
    pub fn new(fd: RawFd) -> IntrActor {
        IntrActor {
            ptr: UnsafeBoxedCell::new(Entry {
                fd: fd,
                intr: true,
                input: Op::default(),
                output: Op::default(),
            })
        }
    }

    pub fn set_intr(&self, io: &IoService) {
        io.0.react.epoll_ctl(unsafe { &*self.ptr.get() }, EPOLL_CTL_ADD, EPOLLIN);
    }

    pub fn unset_intr(&self, io: &IoService) {
        io.0.react.epoll_ctl(unsafe { &*self.ptr.get() }, EPOLL_CTL_DEL, 0);
    }
}

impl AsRawFd for IntrActor {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { self.ptr.get() }.fd
    }
}

impl Drop for IntrActor {
    fn drop(&mut self) {
        let ptr = unsafe { self.ptr.get() };
        libc_ign!(close(ptr.fd));
    }
}


pub struct IoActor {
    io: IoService,
    ptr: UnsafeBoxedCell<Entry>,
}

impl IoActor {
    pub fn new(io: &IoService, fd: RawFd) -> IoActor {
        let ptr = UnsafeBoxedCell::new(Entry {
            fd: fd,
            intr: false,
            input: Op::default(),
            output: Op::default(),
        });
        io.0.react.register(unsafe { ptr.get() });
        io.0.react.epoll_ctl(unsafe { ptr.get() }, EPOLL_CTL_ADD, EPOLLIN | EPOLLOUT | EPOLLET);
        IoActor { io: io.clone(), ptr: ptr }
    }

    pub fn add_input(&self, callback: Callback, try_again: bool) {
        match self.io.0.react.add_op(&mut unsafe { self.ptr.get() }.input, callback, try_again) {
            Ok(Some(callback)) =>
                self.io.0.post(|io| callback(io, READY)),
            Err(callbacks) =>
                for callback in callbacks {
                    self.io.post(|io| callback(io, CANCELED));
                },
            _ => (),
        }
    }

    pub fn add_output(&self, callback: Callback, try_again: bool) {
        match self.io.0.react.add_op(&mut unsafe { self.ptr.get() }.output, callback, try_again) {
            Ok(Some(callback)) =>
                self.io.0.post(|io| callback(io, READY)),
            Err(cbs) =>
                for callback in cbs {
                    self.io.post(|io| callback(io, CANCELED));
                },
            _ => (),
        }
    }

    pub fn next_input(&self) {
        match self.io.0.react.next_op(&mut unsafe { self.ptr.get() }.input) {
            Some(Ok(callback)) =>
                self.io.post(|io| callback(io, READY)),
            Some(Err(cbs)) =>
                for callback in cbs {
                    self.io.post(|io| callback(io, CANCELED));
                },
            _ => (),
        }
    }

    pub fn next_output(&self) {
        match self.io.0.react.next_op(&mut unsafe { self.ptr.get() }.output) {
            Some(Ok(callback)) =>
                self.io.post(|io| callback(io, READY)),
            Some(Err(callbacks)) =>
                for callback in callbacks {
                    self.io.post(|io| callback(io, CANCELED));
                },
            _ => (),
        }
    }

    pub fn del_input(&self) -> Vec<Callback> {
        self.io.0.react.del_ops(&mut unsafe { self.ptr.get() }.input)
    }

    pub fn del_output(&self) -> Vec<Callback> {
        self.io.0.react.del_ops(&mut unsafe { self.ptr.get() }.output)
    }
}

impl IoObject for IoActor {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}

impl AsRawFd for IoActor {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { self.ptr.get() }.fd
    }
}

impl Drop for IoActor {
    fn drop(&mut self) {
        let ptr = unsafe { self.ptr.get() };
        self.io.0.react.epoll_ctl(ptr, EPOLL_CTL_DEL, 0);
        self.io.0.react.unregister(ptr);
        libc_ign!(close(ptr.fd));
    }
}
