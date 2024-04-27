use crate::error::OsError;
use libc;
use std::os::fd::{OwnedFd, FromRawFd, AsRawFd};
use std::time::Duration;
use std::future::Future;
use std::task::{Poll, Context};
use std::pin::Pin;
use std::mem::MaybeUninit;

pub struct Epoll {
    epfd: OwnedFd,
}


impl Epoll {
    pub fn new() -> Result<Self, OsError> {
        unsafe {
            match libc::epoll_create1(libc::EPOLL_CLOEXEC) {
                -1 => Err(OsError::last()),
                epfd => Ok(Epoll { epfd: OwnedFd::from_raw_fd(epfd) })
            }
        }
    }

    fn epoll_ctl(&self, soc: &OwnedFd, op: i32) {
        let mut event = libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLOUT) as u32,
            u64: 0,

        };
        unsafe {
            match libc::epoll_ctl(
                self.epfd.as_raw_fd(),
                op,
                soc.as_raw_fd(),
                &mut event,
            ) {
                _ => {},
            }
        }
    }

    pub fn add_socket(&self, soc: &OwnedFd) {
        self.epoll_ctl(soc, libc::EPOLL_CTL_ADD)
    }

    pub fn del_socket(&self, soc: &OwnedFd) {
        self.epoll_ctl(soc, libc::EPOLL_CTL_DEL)
    }

    fn epoll_wait(&self) -> Result<(), OsError> {
        let mut events = MaybeUninit::<[libc::epoll_event; 128]>::uninit();
        unsafe {
            match libc::epoll_wait(
                self.epfd.as_raw_fd(),
                events.as_mut_ptr().cast(),
                128,
                0,
            ) {
                -1 => Err(OsError::last()),
                len => {
                    let events = events.assume_init();
                    Ok(())
                }
            }
        }
    }
}

impl Future for Epoll {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let _ = self.epoll_wait();
        Poll::Pending
    }
}
