//

use libc_;

pub struct LegacyMutex {
    mutex: libc_::pthread_mutex_t,
}

impl LegacyMutex {
    pub fn new() -> Self {
        LegacyMutex {
            mutex: libc_::PTHREAD_MUTEX_INITIALIZER,
        }
    }

    pub fn lock(&self) {
        unsafe {
            libc_::pthread_mutex_lock(&self.mutex as *const _ as *mut _);
        }
    }

    pub fn unlock(&self) {
        unsafe {
            libc_::pthread_mutex_unlock(&self.mutex as *const _ as *mut _);
        }
    }
}
