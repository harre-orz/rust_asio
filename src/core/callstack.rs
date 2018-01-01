use ffi::TssPtr;

use std::ptr;
use std::cmp::Eq;
use std::ops::{Deref, DerefMut};

lazy_static! {
    static ref TOP: TssPtr<()> = TssPtr::new().unwrap();
}


pub struct ThreadCallStack<K, V> {
    key: *const K,
    next: *mut ThreadCallStack<K, V>,
    value: V,
}

impl<K: Eq, V> ThreadCallStack<K, V> {
    pub fn new(key: &K, value: V) -> Self {
        ThreadCallStack {
            key: key,
            next: ptr::null_mut(),
            value: value,
        }
    }

    pub fn init(&mut self) {
        debug_assert!( self.next.is_null() );
        self.next = TOP.get() as *mut _;
        TOP.set(self as *mut _ as *mut _);
    }

    pub fn callstack<'a>(key: &'a K) -> Option<&'a mut Self> {
        let mut ptr = TOP.get() as *mut Self;
        unsafe {
            while !ptr.is_null() {
                if key.eq( &*(*ptr).key ) {
                    return Some(&mut *ptr)
                }
                ptr = (*ptr).next;
            }
        }
        None
    }
}

impl<K, V> Drop for ThreadCallStack<K, V> {
    fn drop(&mut self) {
        debug_assert!( !self.next.is_null() );
        TOP.set(self.next as *mut _)
    }
}

impl<K, V> Deref for ThreadCallStack<K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<K, V> DerefMut for ThreadCallStack<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}


use super::{IoContext, AsIoContext};

unsafe impl<K: AsIoContext, V> AsIoContext for ThreadCallStack<K, V> {
    fn as_ctx(&self) -> &IoContext {
        unsafe { &*self.key }.as_ctx()
    }
}
