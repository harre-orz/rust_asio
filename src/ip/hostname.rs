use std::io;
use std::mem;
use std::ffi::CStr;
use libc::{self, c_char};

/// Get the current host name.
///
/// # Examples
///
/// ```
/// use asyncio::ip::host_name;
///
/// println!("{}", host_name().unwrap());
/// ```
pub fn host_name() -> io::Result<String> {
    gethostname()
}

fn gethostname() -> io::Result<String> {
    let mut name: [c_char; 65] = unsafe { mem::uninitialized() };
    libc_try!(libc::gethostname(name.as_mut_ptr(), mem::size_of_val(&name)));
    let cstr = unsafe { CStr::from_ptr(name.as_ptr()) };
    Ok(String::from(cstr.to_str().unwrap()))
}

#[test]
fn test_host_name() {
    host_name().unwrap();
}
