use crate::error::{OsError, Result};
use std::ffi::CString;

pub struct Iface {
    ifi: libc::c_uint,
}

impl Iface {
    pub fn new(if_name: &str) -> Result<Iface> {
        if let Ok(if_name) = CString::new(if_name) {
            unsafe {
                match libc::if_nametoindex(if_name.as_ptr().cast()) {
                    0 => Err(OsError::last()),
                    ifi => Ok(Self { ifi: ifi }),
                }
            }
        } else {
            Err(OsError::NO_SUCH_DEVICE)
        }
    }

    pub const unsafe fn from_raw(ifi: libc::c_uint) -> Self {
        Self { ifi: ifi }
    }

    pub const fn as_raw(&self) -> libc::c_uint {
        self.ifi
    }
}
