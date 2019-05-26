use std::io;
use std::marker::PhantomData;
use kernel32::{TlsAlloc, TlsFree, TlsGetValue, TlsSetValue};
use winapi::DWORD;

const TLS_OUT_OF_INDEXES: DWORD = 0xffffffff;

pub struct WinTssPtr<T> {
    tss_key: DWORD,
    _marker: PhantomData<T>,
}

impl<T> Drop for WinTssPtr<T> {
    fn drop(&mut self) {
        unsafe { TlsFree(self.tss_key) };
    }
}

impl<T> WinTssPtr<T> {
    pub fn new() -> io::Result<Self> {
        let tss_key = unsafe { TlsAlloc() };
        if tss_key == TLS_OUT_OF_INDEXES {
            return Err(io::Error::last_os_error());
        }
        Ok(WinTssPtr {
            tss_key: tss_key,
            _marker: PhantomData,
        })
    }

    pub fn get(&self) -> *mut T {
        unsafe { TlsGetValue(self.tss_key) as *mut _ }
    }

    pub fn set(&self, ptr: *mut T) {
        unsafe { TlsSetValue(self.tss_key, ptr as _) };
    }
}

unsafe impl<T> Send for WinTssPtr<T> {}

unsafe impl<T> Sync for WinTssPtr<T> {}
