//

use super::mutex::LegacyMutex;
use super::{SocketContext, TimerQueue};
use error::{ErrorCode, SUCCESS};
use libc;
use socket_base::NativeHandle;
use std::mem;

pub type ReactorCallback = fn(&Reactor, &mut SocketContext, i32);

pub struct Reactor {
    epfd: NativeHandle,
    pub mutex: LegacyMutex,
}

impl Reactor {
    pub fn new() -> Result<Self, ErrorCode> {
        let epfd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if epfd >= 0 {
            Ok(Reactor {
                epfd: epfd,
                mutex: LegacyMutex::new(),
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
                socket_ctx.native_handle(),
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
                socket_ctx.native_handle(),
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
        self.epoll_add(socket_ctx, libc::EPOLLIN | libc::EPOLLOUT | libc::EPOLLET)
    }

    pub fn deregister_socket(&self, socket_ctx: &SocketContext) {
        self.epoll_del(socket_ctx)
    }

    pub fn poll(&self, timer_queue: &mut TimerQueue, timeout: i32) {
        let mut events: [libc::epoll_event; 128] = unsafe { mem::uninitialized() };
        let len = unsafe {
            libc::epoll_wait(self.epfd, events.as_mut_ptr(), events.len() as i32, timeout)
        };
        timer_queue.get_ready_timers(self);
        if len > 0 {
            for ev in &events[..(len as usize)] {
                let socket_ctx = unsafe { &mut *(ev.u64 as *mut SocketContext) };
                (socket_ctx.callback)(self, socket_ctx, ev.events as i32);
            }
        }
    }

    pub fn callback_interrupter(&self, socket_ctx: &mut SocketContext, events: i32) {
        if (events & libc::EPOLLIN) != 0 {
            let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
            let _ = unsafe {
                libc::read(
                    socket_ctx.native_handle(),
                    buf.as_mut_ptr() as *mut _,
                    buf.len(),
                )
            };
        }
    }

    pub fn callback_socket(&self, socket_ctx: &mut SocketContext, events: i32) {
        if (events & (libc::EPOLLERR | libc::EPOLLHUP)) != 0 {
            let err = ErrorCode::socket_error(socket_ctx.native_handle());
            socket_ctx.callback_readable(self, err);
            socket_ctx.callback_writable(self, err);
            return;
        }
        if (events & libc::EPOLLIN) != 0 {
            socket_ctx.callback_readable(self, SUCCESS);
        }
        if (events & libc::EPOLLOUT) != 0 {
            socket_ctx.callback_writable(self, SUCCESS);
        }
    }
}

impl Drop for Reactor {
    fn drop(&mut self) {
        let _ = unsafe { libc::close(self.epfd) };
    }
}
