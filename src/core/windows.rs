pub(crate) struct Handle(Foundation::HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = Foundation::CloseHandle(self.0);
        }
    }
}

pub(crate) trait AsHandle {
    unsafe fn as_raw_handle(&self) -> Foundation::HANDLE;
}

impl Handle {
    pub(crate) unsafe fn new_unchecked(handle: Foundation::HANDLE) -> Self {
        Self(handle)
    }

    pub(crate) fn write(&self, bytes: &[u8]) -> Result<usize> {
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

    pub(crate) fn read(&self, bytes: &mut [u8]) -> Result<usize> {
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

    pub(crate) fn pipe() -> Result<(Self, Self)> {
        let mut read = ptr::null_mut();
        let mut write = ptr::null_mut();

        unsafe {
            match Pipes::CreatePipe(&mut read, &mut write, ptr::null_mut(), 0) {
                0 => Err(OsError::last()),
                _ => Ok((Self(read), Self(write))),
            }
        }
    }
}

impl AsHandle for Handle {
    unsafe fn as_raw_handle(&self) -> Foundation::HANDLE {
        self.0
    }
}
