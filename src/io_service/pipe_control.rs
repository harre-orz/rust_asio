use std::sync::Mutex;
use libc::{c_int, c_void, write, timespec};
use super::{IoService, IntrActor, RawFd, AsRawFd};
use clock::Expiry;

#[cfg(target_os = "linux")]
unsafe fn pipe(pipefd: *mut c_int) -> c_int {
    use libc::{O_CLOEXEC, pipe2};
    pipe2(pipefd, O_CLOEXEC)
}

#[cfg(target_os = "macos")]
unsafe fn pipe(pipefd: *mut c_int) -> c_int {
    use std::slice;
    use libc::{self, FD_CLOEXEC, fcntl};

    let ec = libc::pipe(pipefd);
    if ec != 0 {
        return ec;
    }
    for &fd in slice::from_raw_parts(pipefd, 2) {
        fcntl(fd, FD_CLOEXEC);
    }
    0
}

struct ControlData {
    polling: bool,
    pipe_rfd: IntrActor,
    pipe_wfd: IntrActor,
    expiry: Expiry,
}

pub struct Control {
    mutex: Mutex<ControlData>,
}

impl Control {
    pub fn new() -> Control {
        let mut pipefd = [0; 2];
        libc_unwrap!(pipe(pipefd.as_mut_ptr()));
        Control {
            mutex: Mutex::new(ControlData {
                polling: false,
                pipe_rfd: IntrActor::new(pipefd[0]),
                pipe_wfd: IntrActor::new(pipefd[1]),
                expiry: Expiry::default(),
            })
        }
    }

    pub fn start(&self, io: &IoService) -> bool {
        let mut ctrl = self.mutex.lock().unwrap();
        if ctrl.polling {
            false
        } else {
            ctrl.polling = true;
            ctrl.pipe_rfd.set_intr(io);
            true
        }
    }

    pub fn stop(&self, io: &IoService) {
        let mut ctrl = self.mutex.lock().unwrap();
        ctrl.polling = false;
        ctrl.pipe_rfd.unset_intr(io);
    }

    pub fn interrupt(&self) {
        let ctrl = self.mutex.lock().unwrap();
        do_interrupt(ctrl.pipe_wfd.as_raw_fd());
    }

    pub fn reset_timeout(&self, expiry: Expiry) {
        let mut ctrl = self.mutex.lock().unwrap();
        ctrl.expiry = expiry;
        do_interrupt(ctrl.pipe_wfd.as_raw_fd());
    }

    // 満了時間と現在時刻との差を返す.
    // データは reactor に依存する
    pub fn wait_duration(&self, max: i32) -> timespec {
        // TODO: 満了時間と現在時刻との差を返す.
        timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }
    }
}

fn do_interrupt(fd: RawFd) {
    let buf = [1,0,0,0,0,0,0,0];
    libc_ign!(write(fd, buf.as_ptr() as *const c_void, buf.len()));
}

#[test]
fn test_pipe() {
    let ctrl = Control::new();
    ctrl.interrupt();
    ctrl.reset_timeout(Expiry::now());
}
