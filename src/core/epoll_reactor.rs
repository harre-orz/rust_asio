use super::{NativeHandleType, Expiry};
use std::io;
use std::mem;
use std::cmp;
use std::boxed::FnBox;
use std::sync::{Arc, Mutex, Condvar};
use std::time::Duration;
use libc::*;

const EPOLL_CLOEXEC: c_int = O_CLOEXEC;

extern {
    #[cfg_attr(target_os = "linux", link_name = "epoll_create1")]
    fn epoll_create1(flags: c_int) -> c_int;
}

pub type EpollHandler = Box<FnBox(io::Result<NativeHandleType>) + Send + 'static>;

struct EpollOp {
    callback: EpollHandler,
}

struct EpollObject {
    fd: NativeHandleType,
    intr: bool,
    in_op: Option<EpollOp>,
    out_op: Option<EpollOp>,
}

trait EpollTag {
    fn in_event(ev: &epoll_event) -> bool;
    fn swap_op(ptr: &mut EpollObject, op: &mut Option<EpollOp>);
    fn ctrl_mod(ptr: &EpollObject, ev: &mut epoll_event) -> i32;
}

struct EpollIn;
impl EpollTag for EpollIn {
    fn in_event(ev: &epoll_event) -> bool {
        ev.events & EPOLLIN as u32 != 0
    }

    fn swap_op(ptr: &mut EpollObject, op: &mut Option<EpollOp>) {
        mem::swap(&mut ptr.in_op, op);
    }

    fn ctrl_mod(ptr: &EpollObject, ev: &mut epoll_event) -> i32 {
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

    fn ctrl_mod(ptr: &EpollObject, ev: &mut epoll_event) -> i32{
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
    fd: NativeHandleType,
    polling: bool,
}

impl EpollFd {
    fn ctrl_add(&mut self, ptr: &EpollObject) {
        if ptr.intr {
            let mut ev = epoll_event {
                events: (EPOLLIN) as u32,
                u64: unsafe { mem::transmute(ptr) },
            };
            unsafe { epoll_ctl(self.fd, EPOLL_CTL_ADD, ptr.fd, &mut ev) };
        }
    }

    fn ctrl_del(&mut self, ptr: &EpollObject) {
        if ptr.intr {
            let mut ev = epoll_event {
                events: 0,
                u64: unsafe { mem::transmute(ptr) },
            };
            unsafe { epoll_ctl(self.fd, EPOLL_CTL_DEL, ptr.fd, &mut ev) };
        }
    }

    fn ctrl_mod<T: EpollTag>(&mut self, ptr: &EpollObject, tag: T) {
        if !ptr.intr {
            let mut ev = epoll_event {
                events: (EPOLLET) as u32,
                u64: unsafe { mem::transmute(ptr) },
            };
            let op = T::ctrl_mod(ptr, &mut ev);
            unsafe { epoll_ctl(self.fd, op, ptr.fd, &mut ev) };
        }
    }
}

impl Drop for EpollFd {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

#[derive(Clone)]
pub struct EpollReactor {
    epoll: Arc<(Mutex<EpollFd>, Condvar)>,
}

impl EpollReactor {
    pub fn new() -> io::Result<EpollReactor> {
        let epoll_fd = libc_try!(epoll_create1(EPOLL_CLOEXEC));
        Ok(EpollReactor {
            epoll: Arc::new((Mutex::new(EpollFd {
                fd: epoll_fd,
                polling: false,
            }), Condvar::new())),
        })
    }

    pub fn poll(&self, expiry: &Expiry, vec: &mut Vec<(EpollHandler, NativeHandleType)>) {
        let condvar = &self.epoll.1;
        let epoll_fd = {
            let mut epoll = self.epoll.0.lock().unwrap();
            while epoll.polling {
                epoll = condvar.wait(epoll).unwrap();
            }
            epoll.polling = true;
            epoll.fd
        };
        let timeout = expiry.wait_duration_msec(Duration::new(5, 0)) as i32;
        let mut events: [epoll_event; 128] = unsafe { mem::uninitialized() };
        let n = cmp::max(0, unsafe {
            epoll_wait(epoll_fd, events.as_mut_ptr(), events.len() as i32, timeout)
        }) as usize;
        for ev in &events[..n] {
            self.do_event(&ev, EpollIn, vec);
            self.do_event(&ev, EpollOut, vec);
        }

        let mut epoll = self.epoll.0.lock().unwrap();
        epoll.polling = false;
        condvar.notify_one();
    }

    fn do_event<Tag: EpollTag>(&self, ev: &epoll_event, tag: Tag, vec: &mut Vec<(EpollHandler, NativeHandleType)>) {
        if Tag::in_event(ev) {
            let ptr: &mut EpollObject = unsafe { mem::transmute(ev.u64) };
            if let Some(callback) = self.do_reset(ptr, tag, None) {
                vec.push((callback, ptr.fd))
            }
        }
    }

    fn do_reset<Tag: EpollTag>(&self, ptr: &mut EpollObject, tag: Tag, mut opt: Option<EpollOp>) -> Option<EpollHandler> {
        let mut epoll = self.epoll.0.lock().unwrap();
        Tag::swap_op(ptr, &mut opt);
        epoll.ctrl_mod(ptr, tag);
        if let Some(old_op) = opt {
            Some(old_op.callback)
        } else {
            None
        }
    }

    fn ctrl_add(&self, ptr: &EpollObject) {
        let mut epoll = self.epoll.0.lock().unwrap();
        epoll.ctrl_add(ptr)
    }

    fn ctrl_del(&self, ptr: &EpollObject) {
        let mut epoll = self.epoll.0.lock().unwrap();
        epoll.ctrl_del(ptr)
    }
}

pub struct EpollIoEvent {
    epoll_sv: EpollReactor,
    epoll_ptr: Box<EpollObject>,
}

impl EpollIoEvent {
    pub fn new(sv: &EpollReactor, fd: NativeHandleType) -> EpollIoEvent {
        EpollIoEvent {
            epoll_sv: sv.clone(),
            epoll_ptr: Box::new(EpollObject {
                fd: fd,
                intr: false,
                in_op: None,
                out_op: None,
            }),
        }
    }

    pub fn set_in(&mut self, callback: EpollHandler) -> Option<EpollHandler> {
        self.epoll_sv.clone().do_reset(&mut self.epoll_ptr, EpollIn, Some(EpollOp { callback: callback }))
    }

    pub fn set_out(&mut self, callback: EpollHandler) -> Option<EpollHandler> {
        self.epoll_sv.clone().do_reset(&mut self.epoll_ptr, EpollOut, Some(EpollOp { callback: callback }))
    }

    pub fn unset_in(&mut self) -> Option<EpollHandler> {
        self.epoll_sv.clone().do_reset(&mut self.epoll_ptr, EpollIn, None)
    }

    pub fn unset_out(&mut self) -> Option<EpollHandler> {
        self.epoll_sv.clone().do_reset(&mut self.epoll_ptr, EpollOut, None)
    }
}

impl Drop for EpollIoEvent {
    fn drop(&mut self) {
        let _ = self.unset_in();
        let _ = self.unset_out();
    }
}

pub struct EpollIntrEvent {
    epoll_sv: EpollReactor,
    epoll_ptr: Box<EpollObject>,
}

impl EpollIntrEvent {
    pub fn new(sv: &EpollReactor, fd: NativeHandleType) -> EpollIntrEvent {
        let ev = EpollIntrEvent {
            epoll_sv: sv.clone(),
            epoll_ptr: Box::new(EpollObject {
                fd: fd,
                intr: true,
                in_op: None,
                out_op: None,
            })
        };
        sv.ctrl_add(&ev.epoll_ptr);
        ev
    }
}

impl Drop for EpollIntrEvent {
    fn drop(&mut self) {
        self.epoll_sv.ctrl_del(&self.epoll_ptr);
    }
}

#[test]
fn test_epoll_set_unset() {
    use std::thread;

    let sv = EpollReactor::new().unwrap();
    let mut ev = EpollIoEvent::new(&sv, 0);
    assert!(ev.unset_in().is_none());
    assert!(ev.unset_out().is_none());
    assert!(ev.set_in(Box::new(|_| {})).is_none());
    assert!(ev.set_in(Box::new(|_| {})).is_some());
    assert!(ev.set_out(Box::new(|_| {})).is_none());
    assert!(ev.set_out(Box::new(|_| {})).is_some());
    thread::spawn(move || {
        assert!(ev.unset_in().is_some());
        assert!(ev.unset_out().is_some());
        assert!(ev.unset_in().is_none());
        assert!(ev.unset_out().is_none());
     }).join();
}
