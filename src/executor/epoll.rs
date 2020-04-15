//

use super::{SocketContext, IoContext, Intr};
use error::{ErrorCode, SUCCESS};
use libc;
use socket_base::NativeHandle;
use std::mem::MaybeUninit;

pub type ReactorCallback = fn(&SocketContext, i32, &IoContext);

pub struct Reactor {
    epfd: NativeHandle,
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

    fn epoll_add(&self, socket_ctx: &SocketContext, events: i32) {
        let mut eev = libc::epoll_event {
            events: events as u32,
            u64: socket_ctx as *const _ as u64,
        };
        let _ = unsafe {
            libc::epoll_ctl(
                self.epfd,
                libc::EPOLL_CTL_ADD,
                socket_ctx.handle,
                &mut eev,
            )
        };
    }

    fn epoll_del(&self, socket_ctx: &SocketContext) {
        let mut eev = libc::epoll_event {
            events: 0,
            u64: socket_ctx as *const _ as u64,
        };
        let _ = unsafe {
            libc::epoll_ctl(
                self.epfd,
                libc::EPOLL_CTL_DEL,
                socket_ctx.handle,
                &mut eev,
            )
        };
    }

    pub fn register_interrupter(&self, socket_ctx: &SocketContext) {
        self.epoll_add(socket_ctx, libc::EPOLLIN | libc::EPOLLET)
    }

    pub fn deregister_interrupter(&self, socket_ctx: &SocketContext) {
        self.epoll_del(socket_ctx)
    }

    pub fn register_socket(&self, socket_ctx: &SocketContext) {
        println!("register socket {:p}", socket_ctx);
        self.epoll_add(socket_ctx, libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET)
    }

    pub fn deregister_socket(&self, socket_ctx: &SocketContext) {
        self.epoll_del(socket_ctx)
    }

    pub fn poll(&self, ctx: &IoContext, intr: &Intr) {
        let timeout = intr.wait_duration();
        let mut events: [libc::epoll_event; 128] = unsafe { MaybeUninit::uninit().assume_init() };
        let n = unsafe {
            libc::epoll_wait(self.epfd, events.as_mut_ptr(), events.len() as i32, timeout)
        };
        if n > 0 {
            for ev in &events[..(n as usize)] {
                let socket_ctx = unsafe { &*(ev.u64 as *const SocketContext) };
                (socket_ctx.callback)(socket_ctx, ev.events as i32, ctx);
            }
        }
    }
}

pub fn callback_intr(socket_ctx: &SocketContext, events: i32, _: &IoContext) {
    if (events & libc::EPOLLIN) != 0 {
        let mut buf: [u8; 8] = unsafe { MaybeUninit::uninit().assume_init() };
        let _ = unsafe {
            libc::read(
                socket_ctx.handle,
                buf.as_mut_ptr() as *mut _,
                buf.len(),
            )
        };
    }
}

pub fn callback_socket(socket_ctx: &SocketContext, events: i32, ctx: &IoContext) {
    if (events & (libc::EPOLLERR | libc::EPOLLHUP)) != 0 {
        let err = ErrorCode::socket_error(socket_ctx.handle);
        ctx.read_callback(socket_ctx, err);
        ctx.write_callback(socket_ctx, err);
        return;
    }
    if (events & libc::EPOLLIN) != 0 {
        ctx.read_callback(socket_ctx, SUCCESS);
    }
    if (events & libc::EPOLLOUT) != 0 {
        ctx.write_callback(socket_ctx, SUCCESS);
    }
}

impl Drop for Reactor {
    fn drop(&mut self) {
        let _ = unsafe { libc::close(self.epfd) };
    }
}
