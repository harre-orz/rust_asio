use std::io;
use std::mem;
use std::marker::PhantomData;
use libc::{pthread_key_t, pthread_key_create, pthread_key_delete,
           pthread_getspecific, pthread_setspecific};

pub struct PthreadTssPtr<T> {
    tss_key: pthread_key_t,
    _marker: PhantomData<T>,
}

impl<T> Drop for PthreadTssPtr<T> {
    fn drop(&mut self) {
        libc_ign!(pthread_key_delete(self.tss_key));
    }
}

impl<T> PthreadTssPtr<T> {
    pub fn new() -> io::Result<Self> {
        let mut tss_key: pthread_key_t = unsafe { mem::uninitialized() };
        libc_try!(pthread_key_create(&mut tss_key, None));
        Ok(PthreadTssPtr {
            tss_key: tss_key,
            _marker: PhantomData,
        })
     }

    pub fn get(&self) -> *mut T {
        unsafe { pthread_getspecific(self.tss_key) as *mut _ }
    }

    pub fn set(&self, ptr: *mut T) {
        unsafe { pthread_setspecific(self.tss_key, ptr as *mut _) };
    }
}

unsafe impl<T> Send for PthreadTssPtr<T> { }

unsafe impl<T> Sync for PthreadTssPtr<T> { }
