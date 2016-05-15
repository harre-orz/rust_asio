use libc;

macro_rules! libc_try {
    ($expr:expr) => (match unsafe { $expr } {
        rc if rc >= 0 => rc,
        _ => return Err(io::Error::last_os_error()),
    })
}

extern {
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    fn errno_location() -> *mut libc::c_int;
}

pub fn errno() -> i32 {
    unsafe { *errno_location() }
}
