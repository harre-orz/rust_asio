use super::Deadline;
use crate::error::Result;
use crate::socket::{Fd, Socket};
use std::cell::Cell;

#[cfg(unix)]
mod ffi {
    use crate::error::{OsError, Result};
    use crate::socket::{Fd, Socket};
    use std::mem;
    use std::mem::MaybeUninit;

    pub(super) fn pipe() -> Result<(Socket, Socket)> {
        unsafe {
            let mut fds: [MaybeUninit<libc::c_int>; 2] = [const { MaybeUninit::uninit() }; 2];
            match libc::pipe2(fds[0].as_mut_ptr(), libc::O_CLOEXEC) {
                -1 => Err(OsError::last()),
                _ => {
                    let fds: [libc::c_int; 2] = mem::transmute(fds);
                    let fd1 = Fd::new_unchecked(fds[0]);
                    let fd2 = Fd::new_unchecked(fds[1]);
                    Ok((Socket::from_raw_fd(fd1), Socket::from_raw_fd(fd2)))
                }
            }
        }
    }
}

#[cfg(windows)]
mod ffi {
    pub(super) fn pipe() -> Result<(crate::socket::unix::Fd, crate::socket::unix::Fd)> {}
}

pub struct Pipe {
    rfd: Socket,
    wfd: Socket,
    timer: Cell<Deadline>,
}

impl Pipe {
    pub fn new() -> Result<Self> {
        let (rfd, wfd) = ffi::pipe()?;
        Ok(Pipe {
            rfd: rfd,
            wfd: wfd,
            timer: Cell::new(Deadline::now()),
        })
    }

    pub(super) fn as_fd(&self) -> &Fd {
        self.as_fd()
    }

    pub(super) unsafe fn as_raw_fd(&self) -> libc::c_int {
        unsafe { self.rfd.as_raw_fd() }
    }

    #[cfg(feature = "poll_epoll")]
    pub(super) fn timeout_epoll(&self) -> i32 {
        self.timer.get().as_relative_millis()
    }

    #[cfg(feature = "poll_kqueue")]
    pub(super) fn timeout_kqueue(&self) -> libc::timespec {
        self.timer.get().as_relative_timespec()
    }

    #[cfg(feature = "poll_select")]
    pub(super) fn timeout_select(&self) -> libc::timeval {
        self.timer.get().as_relative_timeval()
    }

    pub(super) fn wake_up_now(&self) {
        self.wfd.write(&[1u8]).unwrap();
    }

    pub(super) fn wake_up_alarm(&self, timer: Deadline) {
        self.timer.set(timer);
    }

    pub(super) fn update_event(&self) {
        self.rfd.read(&mut [0u8; 1]).unwrap();
    }
}

unsafe impl Send for Pipe {}
unsafe impl Sync for Pipe {}
