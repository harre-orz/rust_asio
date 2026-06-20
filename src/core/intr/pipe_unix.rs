use super::Deadline;
use crate::error::OsError;
use crate::primitive::Fd;
use std::mem;
use std::mem::MaybeUninit;

fn pipe() -> Result<(Fd, Fd), OsError> {
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
    rfd: Fd,
    wfd: Fd,
    deadline: Deadline,
}

impl Pipe {
    pub fn new() -> Result<Self, OsError> {
        let (rfd, wfd) = pipe()?;
        Ok(Pipe {
            rfd: rfd,
            wfd: wfd,
            deadline: Deadline::now(),
        })
    }

    pub fn as_fd(&self) -> &Fd {
        &self.rfd
    }

    #[cfg(target_os = "linux")]
    pub fn timeout_epoll(&self) -> i32 {
        self.deadline.as_millis()
    }

    #[cfg(target_os = "macos")]
    pub fn timeout_kqueue(&self) -> libc::timespec {
        self.deadline.as_timespec()
    }

    pub fn wake_up_now(&self) {
        self.wfd.write(&[1u8]).unwrap();
    }

    pub fn wake_up_alarm(&self, deadline: Deadline) {
        self.deadline.set(deadline);
    }

    pub fn update_event(&self) {
        self.rfd.read(&mut [0u8; 1]).unwrap();
    }
}
