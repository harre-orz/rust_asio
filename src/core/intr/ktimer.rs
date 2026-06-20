use crate::error::OsError;

pub(in super::super) struct Ktimer {

}

impl Ktimer {
    pub fn new() -> Result<Self, OsError> {
        Ok(Ktimer {})
    }

    pub fn timeout_kqueue(&self) -> libc::timespec {
        libc::timespec {
            tv_sec: 10000,
            tv_nsec: 0,
        }
    }

    pub fn wake_up_now(&self) {

    }

    pub fn update_event(&self) {

    }
}