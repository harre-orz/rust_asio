use libc::{ECANCELED, c_int};

extern {
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    fn errno_location() -> *mut c_int;
}

pub fn errno() -> i32 {
    unsafe { *errno_location() }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode(pub i32);
pub const READY: ErrorCode = ErrorCode(0);
pub const CANCELED: ErrorCode = ErrorCode(ECANCELED);
