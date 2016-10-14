use std::io;
use std::ptr;
use std::sync::Mutex;
use std::os::unix::io::{AsRawFd};
use libc::{CLOCK_MONOTONIC, EFD_CLOEXEC, c_int, c_void, timespec, eventfd, write};
use IoService;
use super::{IntrActor};

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

fn timerfd_reset<T: AsRawFd>(fd: &T, expiry: timespec) -> io::Result<()> {
    let new_value = itimerspec {
        it_interval: timespec { tv_sec: 0, tv_nsec: 0 },
        it_value: expiry,
    };
    libc_try!(timerfd_settime(fd.as_raw_fd(), TFD_TIMER_ABSTIME, &new_value, ptr::null_mut()));
    Ok(())
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
    pub fn new() -> io::Result<Control> {
        let event_fd = libc_try!(eventfd(0, EFD_CLOEXEC));
        let timer_fd = libc_try!(timerfd_create(CLOCK_MONOTONIC, TFD_CLOEXEC));
        Ok(Control {
            mutex: Mutex::new(ControlData {
                polling: false,
                event_fd: IntrActor::new(event_fd),
                timer_fd: IntrActor::new(timer_fd),
            })
        })
    }

    pub fn start_polling(&self, io: &IoService) -> bool {
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

    pub fn stop_polling(&self, io: &IoService) {
        let mut ctrl = self.mutex.lock().unwrap();
        debug_assert_eq!(ctrl.polling, true);

        ctrl.polling = false;
        ctrl.event_fd.unset_intr(io);
        ctrl.timer_fd.unset_intr(io);
    }

    pub fn stop_interrupt(&self) {
        let ctrl = self.mutex.lock().unwrap();
        if ctrl.polling {
            let buf = [1,0,0,0,0,0,0,0];
            unsafe {
                write(ctrl.event_fd.as_raw_fd(), buf.as_ptr() as *const c_void, buf.len());
            }
        }
    }

    pub fn reset_timeout(&self, expiry: timespec) {
        let ctrl = self.mutex.lock().unwrap();
        if ctrl.polling {
            timerfd_reset(&ctrl.timer_fd, expiry).unwrap();
        }
    }
}
