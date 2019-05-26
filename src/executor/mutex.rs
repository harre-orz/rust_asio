//

use libc_;
use std::mem;

pub struct LegacyMutex {
    mutex: libc_::pthread_mutex_t,
}

impl LegacyMutex {
    pub fn new() -> Self {
        LegacyMutex {
            mutex: unsafe {mem::uninitialized() },
        }
    }

    pub fn lock(&self) {
        unsafe {
            //libc_::pthread_mutex_lock(&self.mutex as *const _ as *mut _);
        }
    }

    pub fn unlock(&self) {
        unsafe {
            //libc_::pthread_mutex_unlock(&self.mutex as *const _ as *mut _);
        }
    }
}
