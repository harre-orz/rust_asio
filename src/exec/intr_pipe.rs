use super::Deadline;
use crate::error::Result;
use std::cell::Cell;
use std::time::Duration;

#[cfg(unix)]
mod ffi {
    use crate::error::{OsError, Result};
    use crate::socket::Fd;
    use std::mem;
    use std::mem::MaybeUninit;

    pub(super) use crate::socket::Fd as NativeHandle;

    pub(crate) fn pipe() -> Result<(Fd, Fd)> {
        let mut fds: [MaybeUninit<libc::c_int>; 2] = [const { MaybeUninit::uninit() }; 2];
        unsafe {
            #[cfg(target_os = "linux")]
            let res = libc::pipe2(fds[0].as_mut_ptr(), libc::O_CLOEXEC);
            #[cfg(target_os = "macos")]
            let res = libc::pipe(fds[0].as_mut_ptr());
            match res {
                -1 => Err(OsError::last()),
                _ => {
                    let fds = mem::transmute::<_, [libc::c_int; 2]>(fds);
                    let fd1 = Fd::new_unchecked(fds[0]);
                    let fd2 = Fd::new_unchecked(fds[1]);
                    #[cfg(target_os = "macos")]
                    fd1.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    fd2.set_cloexec()?;
                    Ok((fd1, fd2))
                }
            }
        }
    }
}

#[cfg(windows)]
mod ffi {
    use crate::error::{OsError, Result};
    use std::ptr;
    use windows_sys::Win32::Foundation;
    use windows_sys::Win32::Storage::FileSystem;
    use windows_sys::Win32::System::Pipes;

    pub(crate) struct Handle(Foundation::HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = Foundation::CloseHandle(self.0);
            }
        }
    }

    impl Handle {
        pub const unsafe fn from_raw_handle(handle: Foundation::HANDLE) -> Self {
            Self(handle)
        }

        pub const unsafe fn as_raw_handle(&self) -> Foundation::HANDLE {
            self.0
        }

        pub fn write(&self, bytes: &[u8]) -> crate::error::Result<usize> {
            let mut len = 0;
            unsafe {
                match FileSystem::WriteFile(
                    self.0,
                    bytes.as_ptr(),
                    bytes.len() as u32,
                    &mut len,
                    ptr::null_mut(),
                ) {
                    0 => Err(OsError::last()),
                    _ => Ok(len as usize),
                }
            }
        }

        pub fn read(&self, bytes: &mut [u8]) -> crate::error::Result<usize> {
            let mut len = 0;
            unsafe {
                match FileSystem::ReadFile(
                    self.0,
                    bytes.as_mut_ptr(),
                    bytes.len() as u32,
                    &mut len,
                    ptr::null_mut(),
                ) {
                    0 => Err(OsError::last()),
                    _ => Ok(len as usize),
                }
            }
        }
    }

    pub(super) fn pipe() -> Result<(Handle, Handle)> {
        let mut read = ptr::null_mut();
        let mut write = ptr::null_mut();

        unsafe {
            match Pipes::CreatePipe(&mut read, &mut write, ptr::null_mut(), 0) {
                0 => Err(OsError::last()),
                _ => Ok((
                    Handle::from_raw_handle(read),
                    Handle::from_raw_handle(write),
                )),
            }
        }
    }
}

pub struct Pipe {
    rfd: ffi::Handle,
    wfd: ffi::Handle,
    timer: Cell<Deadline>,
}

impl Pipe {
    pub fn new() -> Result<Self> {
        let (rfd, wfd) = ffi::pipe()?;
        Ok(Pipe {
            rfd: rfd,
            wfd: wfd,
            timer: Cell::new(Deadline::now()),
        })
    }

    pub(super) const fn as_fd(&self) -> &ffi::Handle {
        &self.rfd
    }

    #[cfg(target_os = "linux")]
    pub(super) fn timeout_epoll(&self) -> i32 {
        self.timer.get().elapsed().as_millis() as i32
    }

    pub(super) fn timeout(&self) -> Duration {
        self.timer.get().elapsed()
    }

    pub(super) fn wake_up_now(&self) {
        self.wfd.write(&[1u8]).unwrap();
    }

    pub(super) fn wake_up_alarm(&self, timer: Deadline) {
        self.timer.set(timer);
    }

    pub(super) fn update_event(&self) {
        self.rfd.read(&mut [0u8; 1]).unwrap();
    }
}

unsafe impl Send for Pipe {}
unsafe impl Sync for Pipe {}
