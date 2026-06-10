use super::Deadline;
use super::Fd;
use crate::error::{OsError, Result};
use std::cell::Cell;
use std::mem;
use std::mem::MaybeUninit;

pub(crate) fn pipe() -> Result<(Fd, Fd)> {
    let mut fds: [MaybeUninit<libc::c_int>; 2] = [const { MaybeUninit::uninit() }; 2];
    unsafe {
        #[cfg(target_os = "linux")]
        let res = libc::pipe2(fds[0].as_mut_ptr(), libc::O_CLOEXEC);
        #[cfg(target_os = "macos")]
        let res = libc::pipe(fds[0].as_mut_ptr());
        match res {
            -1 => Err(OsError::last()),
            _ => {
                let fds = mem::transmute::<_, [libc::c_int; 2]>(fds);
                let fd1 = Fd::new_unchecked(fds[0]);
                let fd2 = Fd::new_unchecked(fds[1]);
                #[cfg(target_os = "macos")]
                fd1.set_cloexec()?;
                #[cfg(target_os = "macos")]
                fd2.set_cloexec()?;
                Ok((fd1, fd2))
            }
        }
    }
}

pub struct Pipe {
    rfd: Fd,
    wfd: Fd,
    timer: Cell<Deadline>,
}

impl Pipe {
    pub(super) fn new() -> Result<Self> {
        let (rfd, wfd) = pipe()?;
        Ok(Pipe {
            rfd: rfd,
            wfd: wfd,
            timer: Cell::new(Deadline::now()),
        })
    }

    pub(super) const fn as_fd(&self) -> &Fd {
        &self.rfd
    }

    #[cfg(target_os = "linux")]
    pub(super) fn timeout_epoll(&self) -> i32 {
        self.timer.get().elapsed().as_millis() as i32
    }

    #[cfg(target_os = "macos")]
    pub(super) fn timeout_kqueue(&self) -> libc::timespec {
        let tv = self.timer.get().elapsed();
        libc::timespec {
            tv_sec: tv.as_secs() as libc::time_t,
            tv_nsec: tv.subsec_nanos() as libc::c_long,
        }
    }

    pub(super) fn wake_up_now(&self) {
        self.wfd.write(&[1u8]).unwrap();
    }

    pub(crate) fn wake_up_alarm(&self, timer: Deadline) {
        self.timer.set(timer);
    }

    pub(super) fn update_event(&self) {
        self.rfd.read(&mut [0u8; 1]).unwrap();
    }
}

unsafe impl Send for Pipe {}
unsafe impl Sync for Pipe {}
