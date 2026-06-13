use crate::error::{OsError, Result};
use crate::primitive::{Deadline, Fd};
use std::cell::Cell;
use std::mem;
use std::mem::MaybeUninit;

fn pipe() -> Result<(Fd, Fd)> {
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
                let fd1 = Fd::from_raw_fd(fds[0]);
                let fd2 = Fd::from_raw_fd(fds[1]);
                #[cfg(target_os = "macos")]
                fd1.set_cloexec()?;
                #[cfg(target_os = "macos")]
                fd2.set_cloexec()?;
                Ok((fd1, fd2))
            }
        }
    }
}

pub(in super::super) struct Pipe {
    wfd: Fd,
    timer: Cell<Deadline>,
}

impl Pipe {
    pub fn new() -> Result<(Self, Fd)> {
        let (rfd, wfd) = pipe()?;
        Ok((Pipe {
            wfd: wfd,
            timer: Cell::new(Deadline::now()),
        }, rfd))
    }

    #[cfg(target_os = "linux")]
    pub fn timeout_epoll(&self) -> i32 {
        self.timer.get().elapsed().as_millis() as i32
    }

    #[cfg(target_os = "macos")]
    pub fn timeout_kqueue(&self) -> libc::timespec {
        let tv = self.timer.get().elapsed();
        libc::timespec {
            tv_sec: tv.as_secs() as libc::time_t,
            tv_nsec: tv.subsec_nanos() as libc::c_long,
        }
    }

    pub fn wake_up_now(&self, rfd: &Fd) {
        self.wfd.write(&[1u8]).unwrap();
    }

    pub fn wake_up_alarm(&self, rfd: &Fd, timer: Deadline) {
        self.timer.set(timer);
    }

    pub fn update_event(&self, rfd: &Fd) {
        rfd.read(&mut [0u8; 1]).unwrap();
    }
}

unsafe impl Send for Pipe {}
unsafe impl Sync for Pipe {}
