use std::mem;
use std::ptr;
use std::collections::VecDeque;
use std::sync::Mutex;
use unsafe_cell::{UnsafeBoxedCell};
use error::{ErrCode, READY, ECANCELED, EAGAIN, EINPROGRESS, getsockerr};
use super::{IoObject, IoService, ThreadInfo, RawFd, AsRawFd, Callback};
use libc::{c_void, close, read, timespec, 
           EV_ADD, EV_DELETE, EV_ERROR, EV_CLEAR, EV_ENABLE, EV_DISPATCH, EVFILT_READ, EVFILT_WRITE, kqueue, kevent};

struct Op {
    ops: VecDeque<Callback>,
    ready: bool,
    canceling: bool,
}

impl Default for Op {
    fn default() -> Self {
        Op {
            ops: VecDeque::new(),
            ready: true,
            canceling: false,
        }
    }
}

struct Entry {
    fd: RawFd,
    intr: bool,
    input: Op,
    output: Op,
}

struct ReactData {
    callback_count: usize,
    registered_entry: Vec<*mut Entry>,
}

unsafe impl Send for ReactData {
}

unsafe impl Sync for ReactData {
}

pub struct Reactor {
    kqueue_fd: RawFd,
    mutex: Mutex<ReactData>,
}

impl Reactor {
    pub fn new() -> Reactor {
        let kqueue_fd = libc_unwrap!(kqueue());
        Reactor {
            kqueue_fd: kqueue_fd,
            mutex: Mutex::new(ReactData {
                callback_count: 0,
                registered_entry: Vec::new(),
            }),
        }
    }

    pub fn poll(&self, timeout: Option<timespec>, io: &IoService, ti: &ThreadInfo) -> usize {
        let tv = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let mut kevs: [kevent; 128] = unsafe { mem::uninitialized() };
        let len = unsafe {
            kevent(self.kqueue_fd, ptr::null(), 0, kevs.as_mut_ptr(), kevs.len() as i32, &tv)
        };

        if len > 0 {
            for kev in &kevs[..len as usize] {
                let ptr = unsafe { &mut *(kev.udata as *mut Entry) };
                if ptr.intr {
                    if kev.filter == EVFILT_READ {
                        let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
                        libc_ign!(read(ptr.fd, buf.as_mut_ptr() as *mut c_void, buf.len()));
                    }
                } else {
                    if (kev.flags & EV_ERROR) != 0 {
                        let ec = getsockerr(ptr.fd);
/*
                        let mut epoll = self.mutex.lock().unwrap();
                        while let Some(callback) = ptr.input.ops.pop_front() {
                            epoll.callback_count -= 1;
                            io.post(move |io| callback(io, ec));
                        }
                        while let Some(callback) = ptr.output.ops.pop_front() {
                            epoll.callback_count -= 1;
                            io.post(move |io| callback(io, ec));
                        }
*/
                    } else {
                        if kev.filter == EVFILT_READ {
                            let mut epoll = self.mutex.lock().unwrap();
                            if let Some(callback) = ptr.input.ops.pop_front() {
                                epoll.callback_count -= 1;
                                ti.push(callback);
                                ptr.input.ready = false;
                            } else {
                                ptr.input.ready = true;
                            }
                        }
                        if kev.filter == EVFILT_WRITE {
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

        let kqueue = self.mutex.lock().unwrap();
        kqueue.callback_count
    }

    pub fn cancel_all(&self, ti: &ThreadInfo) {
        let mut kqueue = self.mutex.lock().unwrap();
        for ptr in &kqueue.registered_entry {
            while let Some(callback) = unsafe { &mut **ptr }.input.ops.pop_front() {
                ti.push(callback);
            }
            while let Some(callback) = unsafe { &mut **ptr }.output.ops.pop_front() {
                ti.push(callback);
            }
        }
        kqueue.callback_count = 0;
    }

    fn register(&self, ptr: &mut Entry) {
        let mut epoll = self.mutex.lock().unwrap();
        epoll.registered_entry.push(ptr)
    }

    fn unregister(&self, ptr: &mut Entry) {
        let mut epoll = self.mutex.lock().unwrap();
        let idx = epoll.registered_entry.iter().position(|&e| unsafe { &*e }.fd == ptr.fd).unwrap();
        epoll.registered_entry.remove(idx);
    }

    fn kevent(&self, ptr: &Entry, flags: u16, filter: i16) {
        let kev = kevent {
            ident: ptr.fd as usize,
            filter: filter,
            flags: flags,
            fflags: 0,
            data: 0,
            udata: ptr as *const _ as *mut c_void,
        };
        libc_ign!(kevent(self.kqueue_fd, &kev, 1, ptr::null_mut(), 0, ptr::null()));
    }

    fn add_op(&self, op: &mut Op, callback: Callback, ec: ErrCode) -> Result<Option<Callback>, Vec<Callback>> {
        let mut kqueue = self.mutex.lock().unwrap();
        if op.canceling && ec == EAGAIN {
            kqueue.callback_count -= op.ops.len();
            op.ops.push_front(callback);
            Err(op.ops.drain(..).collect())
        } else {
            op.canceling = false;
            if op.ready && ec != EINPROGRESS {
                if op.ops.is_empty() || ec == EAGAIN {
                    Ok(Some(callback))
                } else {
                    op.ops.push_back(callback);
                    Ok(op.ops.pop_front())
                }
            } else {
                op.ready = false;
                kqueue.callback_count += 1;
                if ec == EAGAIN {
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

pub struct IntrActor {
    ptr: UnsafeBoxedCell<Entry>,
}

impl IntrActor {
    pub fn new(fd: RawFd) -> IntrActor {
        let ptr = UnsafeBoxedCell::new(Entry {
            fd: fd,
            intr: true,
            input: Op::default(),
            output: Op::default(),
        });
        IntrActor {
            ptr: ptr,
        }
    }

    pub fn set_intr(&self, io: &IoService) {
        io.0.react.kevent(unsafe { self.ptr.get() }, EV_ADD, EVFILT_READ);
    }

    pub fn unset_intr(&self, io: &IoService) {
        io.0.react.kevent(unsafe { self.ptr.get() }, EV_DELETE, EVFILT_READ);
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
        io.0.react.kevent(unsafe { ptr.get() }, EV_ADD, EVFILT_READ);
        io.0.react.kevent(unsafe { ptr.get() }, EV_ADD, EVFILT_WRITE);
        IoActor {
            io: io.clone(),
            ptr: ptr,
        }
    }

    pub fn add_input(&self, callback: Callback, ec: ErrCode) {
        match self.io.0.react.add_op(&mut unsafe { self.ptr.get() }.input, callback, ec) {
            Ok(Some(callback)) =>
                self.io.0.post(|io| callback(io, READY)),
            Err(callbacks) =>
                for callback in callbacks {
                    self.io.post(|io| callback(io, ECANCELED));
                },
            _ => (),
        }
    }

    pub fn add_output(&self, callback: Callback, ec: ErrCode) {
        match self.io.0.react.add_op(&mut unsafe { self.ptr.get() }.output, callback, ec) {
            Ok(Some(callback)) =>
                self.io.0.post(|io| callback(io, READY)),
            Err(callbacks) =>
                for callback in callbacks {
                    self.io.post(|io| callback(io, ECANCELED));
                },
            _ => (),
        }
    }

    pub fn next_input(&self) {
        match self.io.0.react.next_op(&mut unsafe { self.ptr.get() }.input) {
            Some(Ok(callback)) =>
                self.io.post(|io| callback(io, READY)),
            Some(Err(callbacks)) =>
                for callback in callbacks {
                    self.io.post(|io| callback(io, ECANCELED));
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
                    self.io.post(|io| callback(io, ECANCELED));
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

unsafe impl IoObject for IoActor {
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
        //self.io.0.react.kevent(ptr, EV_DELETE, EVFILT_READ);
        self.io.0.react.kevent(ptr, EV_DELETE, EVFILT_WRITE);
        self.io.0.react.unregister(ptr);
        libc_ign!(close(ptr.fd));
    }
}
