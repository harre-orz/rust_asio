use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};
use std::hash::{Hash, Hasher};

pub struct UnsafeRef<T>(*const T);

impl<T> UnsafeRef<T> {
    pub fn new(t: &T) -> Self {
        UnsafeRef(t)
    }

    pub unsafe fn clone(&self) -> Self {
        UnsafeRef(self.0)
    }
}

unsafe impl<T: Send> Send for UnsafeRef<T> {}

unsafe impl<T: Sync> Sync for UnsafeRef<T> {}

impl<T> Deref for UnsafeRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T> DerefMut for UnsafeRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0 as *mut Self::Target) }
    }
}

impl<T: Eq> Eq for UnsafeRef<T> {}

impl<T: PartialEq> PartialEq for UnsafeRef<T> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { (*self.0).eq(&*other.0) }
    }
}

impl<T: Ord> Ord for UnsafeRef<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe { (*self.0).cmp(&*other.0) }
    }
}

impl<T: PartialOrd> PartialOrd for UnsafeRef<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unsafe { (*self.0).partial_cmp(&*other.0) }
    }
}

impl<T> Hash for UnsafeRef<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_usize(self.0 as usize)
    }
}
