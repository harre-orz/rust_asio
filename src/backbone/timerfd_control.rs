use std::io;
use std::ptr;
use std::sync::Mutex;
use std::os::unix::io::AsRawFd;
use libc::{CLOCK_MONOTONIC, O_CLOEXEC, c_int, c_void, timespec, eventfd, write};
use IoService;
use super::{Expiry, IntrActor};

const EFD_CLOEXEC: i32 = O_CLOEXEC;
const TFD_CLOEXEC: i32 = O_CLOEXEC;
//const TFD_TIMER_RELTIME: i32 = 0;
const TFD_TIMER_ABSTIME: i32 = 1;

#[repr(C)]
pub struct itimerspec {

    /// Interval for periodic timer
    pub it_interval: timespec,

    /// Initial expiration
    pub it_value: timespec,
}

extern {
    #[cfg_attr(target_os = "linux", link_name = "timerfd_create")]
    fn timerfd_create(clkid: c_int, flags: c_int) -> c_int;

    #[cfg_attr(target_os = "linux", link_name = "timerfd_settime")]
    fn timerfd_settime(fd: c_int, flags: c_int, new_value: *const itimerspec, old_value: *mut itimerspec) -> c_int;

    // #[cfg_attr(target_os = "linux", link_name = "timerfd_gettime")]
    // fn timerfd_gettime(fd: c_int, cur_value: *mut itimerspec) -> c_int;
}

fn timerfd_reset<T: AsRawFd>(fd: &T, expiry: Expiry) -> io::Result<()> {
    let duration = expiry.wait_duration();
    let new_value = itimerspec {
        it_interval: timespec { tv_sec: 0, tv_nsec: 0 },
        it_value: timespec {
            tv_sec: duration.as_secs() as i64,
            tv_nsec: duration.subsec_nanos() as i64,
        },
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
            unsafe {
                let buf = [1,0,0,0,0,0,0,0];
                write(ctrl.event_fd.as_raw_fd(), buf.as_ptr() as *const c_void, buf.len());
            }
        }
    }

    pub fn reset_timeout(&self, expiry: Expiry) {
        let ctrl = self.mutex.lock().unwrap();
        if ctrl.polling {
            timerfd_reset(&ctrl.timer_fd, expiry).unwrap();
        }
    }
}
