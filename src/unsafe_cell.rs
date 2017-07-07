use std::slice;
use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};
use std::hash::{Hash, Hasher};

pub struct UnsafeBoxedCell<T>(*mut T);

impl<T> UnsafeBoxedCell<T> {
    pub fn new(t: T) -> Self {
        UnsafeBoxedCell(Box::into_raw(Box::new(t)))
    }

    pub unsafe fn from_ref(t: &T) -> Self {
        UnsafeBoxedCell(t as *const _ as *mut _)
    }

    pub fn release(&self) -> Box<T> {
        unsafe { Box::from_raw(self.0) }
    }
}

impl<T> Clone for UnsafeBoxedCell<T> {
    fn clone(&self) -> Self {
        UnsafeBoxedCell(self.0.clone())
    }
}

impl<T: Eq> Eq for UnsafeBoxedCell<T> { }

impl<T: PartialEq> PartialEq for UnsafeBoxedCell<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: Ord> Ord for UnsafeBoxedCell<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: PartialOrd> PartialOrd for UnsafeBoxedCell<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T: Hash> Hash for UnsafeBoxedCell<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T> Deref for UnsafeBoxedCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T> DerefMut for UnsafeBoxedCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}

unsafe impl<T> Send for UnsafeBoxedCell<T> { }


/// コールバックハンドラで参照 &T を転送するために使うセル.
pub struct UnsafeRefCell<T> {
    ptr: *mut T,
}

impl<T> UnsafeRefCell<T> {
    pub fn new(t: &T) -> UnsafeRefCell<T> {
        UnsafeRefCell { ptr: t as *const _ as *mut _ }
    }

    pub unsafe fn as_ref(&self) -> &T {
        &*self.ptr
    }

    pub unsafe fn as_mut(&mut self) -> &mut T {
        &mut *self.ptr
    }
}

unsafe impl<T> Send for UnsafeRefCell<T> { }

/// コールバックハンドラでスライス &[T] を転送するために使うセル.
pub struct UnsafeSliceCell<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> UnsafeSliceCell<T> {
    pub fn new(t: &[T]) -> UnsafeSliceCell<T> {
        UnsafeSliceCell {
            ptr: t.as_ptr() as *mut _,
            len: t.len(),
        }
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        slice::from_raw_parts(self.ptr, self.len)
    }

    pub unsafe fn as_mut_slice(&mut self) -> &mut [T] {
        slice::from_raw_parts_mut(self.ptr, self.len)
    }
}

unsafe impl<T> Send for UnsafeSliceCell<T> { }
