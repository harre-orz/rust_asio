//

use super::{IoContext, Intr, ThreadContext};
use error::{ErrorCode, SUCCESS};
use libc;
use socket_base::{Socket, NativeHandle};
use std::mem::MaybeUninit;

pub struct Reactor {
    epfd: NativeHandle,
}

impl Drop for Reactor {
    fn drop(&mut self) {
        let _ = unsafe { libc::close(self.epfd) };
    }
}

impl Reactor {
    pub fn new() -> Result<Self, ErrorCode> {
        let epfd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if epfd >= 0 {
            Ok(Reactor {
                epfd: epfd,
            })
        } else {
            Err(ErrorCode::last_error())
        }
    }

    fn epoll_add(&self, fd: NativeHandle, events: i32) {
        let mut eev = libc::epoll_event {
            events: events as u32,
            u64: fd as u64,
        };
        let _ = unsafe {
            libc::epoll_ctl(
                self.epfd,
                libc::EPOLL_CTL_ADD,
                fd,
                &mut eev,
            )
        };
    }

    fn epoll_del(&self, fd: NativeHandle) {
        let mut eev = libc::epoll_event {
            events: 0,
            u64: fd as u64,
        };
        let _ = unsafe {
            libc::epoll_ctl(
                self.epfd,
                libc::EPOLL_CTL_DEL,
                fd,
                &mut eev,
            )
        };
    }

    pub fn register_intr(&self, intr: &Intr) {
        self.epoll_add(intr.native_handle(), libc::EPOLLIN | libc::EPOLLET)
    }

    pub fn deregister_intr(&self, intr: &Intr) {
        self.epoll_del(intr.native_handle())
    }

    pub fn register_socket<P, S>(&self, soc: &S)
        where S: Socket<P>
    {
        self.epoll_add(soc.native_handle(), libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET)
    }

    pub fn deregister_socket<P, S>(&self, soc: &S)
        where S: Socket<P>
    {
        self.epoll_del(soc.native_handle())
    }

    pub fn poll(&self, intr: &Intr, ctx: &IoContext, thrd_ctx: &mut ThreadContext) {
        let timeout = intr.wait_duration();
        let mut events: [libc::epoll_event; 128] = unsafe { MaybeUninit::uninit().assume_init() };
        let n = unsafe {
            libc::epoll_wait(self.epfd, events.as_mut_ptr(), events.len() as i32, timeout)
        };

        for ev in &events[..(n as usize)] {
            let fd = ev.u64 as NativeHandle;
            let ev = ev.events as i32;

            if fd == intr.native_handle() {
                callback_intr(fd, ev)
            } else {
                callback_socket(fd, ev, ctx, thrd_ctx)
            }
        }
    }
}

fn callback_intr(fd: NativeHandle, ev: i32) {
    if (ev & libc::EPOLLIN) != 0 {
        let mut buf: [u8; 8] = unsafe { MaybeUninit::uninit().assume_init() };
        let _ = unsafe {
            libc::read(
                fd,
                buf.as_mut_ptr() as *mut _,
                buf.len(),
            )
        };
    }
}

fn callback_socket(fd: NativeHandle, ev: i32, ctx: &IoContext,  thrd_ctx: &mut ThreadContext) {
    if (ev & (libc::EPOLLERR | libc::EPOLLHUP)) != 0 {
        let err = ErrorCode::socket_error(fd);
        ctx.read_callback(fd, err, thrd_ctx);
        ctx.write_callback(fd, err, thrd_ctx);
        return;
    }
    if (ev & libc::EPOLLIN) != 0 {
        ctx.read_callback(fd, SUCCESS, thrd_ctx);
    }
    if (ev & libc::EPOLLOUT) != 0 {
        ctx.write_callback(fd, SUCCESS, thrd_ctx);
    }
}
