use std::io;
use std::ptr;
use std::sync::Mutex;
use libc::{CLOCK_MONOTONIC, EFD_CLOEXEC, c_int, c_void, timespec, eventfd, write};
use clock::Expiry;
use super::{IoService, IntrActor, RawFd, AsRawFd};

#[repr(C)]
pub struct itimerspec {
    pub it_interval: timespec,
    pub it_value: timespec,
}

pub const TFD_CLOEXEC: c_int = 0o2000000;
//pub const TFD_NONBLOCK: c_int = 0o4000;
pub const TFD_TIMER_ABSTIME: c_int = 1 << 0;

extern {
    pub fn timerfd_create(clkid: c_int, flags: c_int) -> c_int;
    pub fn timerfd_settime(fd: c_int,
                           flags: c_int,
                           new_value: *const itimerspec,
                           old_value: *mut itimerspec) -> c_int;
    // pub fn timerfd_gettime(fd: c_int,
    //                        curr_value: *mut itimerspec) -> c_int;
}


struct ControlData {
    polling: bool,
    event_fd: IntrActor,
    timer_fd: IntrActor,
}

pub struct Control {
    mutex: Mutex<ControlData>,
}

impl Control {
    pub fn new() -> Control {
        let event_fd = libc_unwrap!(eventfd(0, EFD_CLOEXEC));
        let timer_fd = libc_unwrap!(timerfd_create(CLOCK_MONOTONIC, TFD_CLOEXEC));
        Control {
            mutex: Mutex::new(ControlData {
                polling: false,
                event_fd: IntrActor::new(event_fd),
                timer_fd: IntrActor::new(timer_fd),
            })
        }
    }

    pub fn start(&self, io: &IoService) -> bool {
        let mut ctrl = self.mutex.lock().unwrap();
        if ctrl.polling {
            false
        } else {
            ctrl.polling = true;
            ctrl.event_fd.set_intr(io);
            ctrl.timer_fd.set_intr(io);
            true
        }
    }

    pub fn stop(&self, io: &IoService) {
        let mut ctrl = self.mutex.lock().unwrap();
        debug_assert_eq!(ctrl.polling, true);

        ctrl.polling = false;
        ctrl.event_fd.unset_intr(io);
        ctrl.timer_fd.unset_intr(io);
    }

    pub fn interrupt(&self) {
        let ctrl = self.mutex.lock().unwrap();
        if ctrl.polling {
            do_interrupt(ctrl.event_fd.as_raw_fd());
        }
    }

    pub fn reset_timeout(&self, expiry: Expiry) {
        let ctrl = self.mutex.lock().unwrap();
        if ctrl.polling {
            timerfd_reset(ctrl.timer_fd.as_raw_fd(), expiry).unwrap();
        }
    }

    pub fn wait_duration(&self, _max: i32) -> i32 {
        -1
    }
}

fn do_interrupt(fd: RawFd) {
    let buf = [1,0,0,0,0,0,0,0];
    libc_ign!(write(fd, buf.as_ptr() as *const c_void, buf.len()));
}

fn timerfd_reset(fd: RawFd, expiry: Expiry) -> io::Result<()> {
    let new_value = itimerspec {
        it_interval: timespec { tv_sec: 0, tv_nsec: 0 },
        it_value: timespec {
            tv_sec: expiry.as_secs() as i64,
            tv_nsec: expiry.subsec_nanos() as i64
        },
    };
    libc_try!(timerfd_settime(fd, TFD_TIMER_ABSTIME, &new_value, ptr::null_mut()));
    Ok(())
}

#[test]
fn test_timerfd() {
    let ctrl = Control::new();
    ctrl.interrupt();
    ctrl.reset_timeout(Expiry::now());
}
