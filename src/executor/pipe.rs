//

use error::ErrorCode;
use libc;
use socket::{pipe, NativeHandle};

pub struct Interrupter {
    rfd: NativeHandle,
    wfd: NativeHandle,
}

impl Interrupter {
    pub fn new() -> Result<Self, ErrorCode> {
        let (rfd, wfd) = pipe()?;
        Ok(Interrupter { rfd: rfd, wfd: wfd })
    }

    pub fn interrupt(&self) {
        let buf = [1];
        let _ = unsafe { libc::write(self.wfd, buf.as_ptr() as *const _, buf.len()) };
    }
}
