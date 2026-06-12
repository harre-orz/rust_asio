use super::Deadline;
use crate::primitive::Fd;
use crate::error::{OsError, Result};
use std::cell::Cell;
use std::mem;
use std::mem::MaybeUninit;

#[cfg(target_os = "linux")]
fn pipe() -> Result<(Fd, Fd)> {
    let mut fds: [MaybeUninit<libc::c_int>; 2] = [const { MaybeUninit::uninit() }; 2];
    unsafe {
        let res = libc::pipe2(fds[0].as_mut_ptr(), libc::O_CLOEXEC);
        match res {
            -1 => Err(OsError::last()),
            _ => {
                let fds = mem::transmute::<_, [libc::c_int; 2]>(fds);
                let fd1 = Fd::from_raw_fd(fds[0]);
                let fd2 = Fd::from_raw_fd(fds[1]);
                Ok((fd1, fd2))
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn pipe() -> Result<(Fd, Fd)> {
    let mut fds: [MaybeUninit<libc::c_int>; 2] = [const { MaybeUninit::uninit() }; 2];
    unsafe {
        let res = libc::pipe(fds[0].as_mut_ptr());
        match res {
            -1 => Err(OsError::last()),
            _ => {
                let fds = mem::transmute::<_, [libc::c_int; 2]>(fds);
                let fd1 = Fd::from_raw_fd(fds[0]);
                let fd2 = Fd::from_raw_fd(fds[1]);
                fd1.set_cloexec()?;
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
    pub(crate) fn new() -> Result<Self> {
        let (rfd, wfd) = pipe()?;
        Ok(Pipe {
            rfd: rfd,
            wfd: wfd,
            timer: Cell::new(Deadline::now()),
        })
    }

    pub(crate) const fn as_fd(&self) -> &Fd {
        &self.rfd
    }

    #[cfg(target_os = "linux")]
    pub(super) fn timeout_epoll(&self) -> i32 {
        self.timer.get().elapsed().as_millis() as i32
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn timeout_kqueue(&self) -> libc::timespec {
        let tv = self.timer.get().elapsed();
        libc::timespec {
            tv_sec: tv.as_secs() as libc::time_t,
            tv_nsec: tv.subsec_nanos() as libc::c_long,
        }
    }

    pub(crate) fn wake_up_now(&self) {
        self.wfd.write(&[1u8]).unwrap();
    }

    pub(crate) fn wake_up_alarm(&self, timer: Deadline) {
        self.timer.set(timer);
    }

    pub(crate) fn update_event(&self) {
        self.rfd.read(&mut [0u8; 1]).unwrap();
    }
}

unsafe impl Send for Pipe {}
unsafe impl Sync for Pipe {}
