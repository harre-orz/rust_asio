use std::slice;

pub struct UnsafeRefCell<T> {
    ptr: *mut T,
}

impl<T> UnsafeRefCell<T> {
    pub fn new(t: &T) -> UnsafeRefCell<T> {
        UnsafeRefCell { ptr: t as *const _ as *mut _}
    }

    pub unsafe fn as_ref(&self) -> &T {
        &*self.ptr
    }

    pub unsafe fn as_mut(&mut self) -> &mut T {
        &mut *self.ptr
    }
}

unsafe impl<T> Send for UnsafeRefCell<T> {
}

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

    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        slice::from_raw_parts_mut(self.ptr as *mut T, self.len)
    }
}

unsafe impl<T> Send for UnsafeSliceCell<T> {
}

pub struct UnsafeStrandCell<T> {
    data: T,
}

impl<T> UnsafeStrandCell<T> {
    pub fn new(data: T) -> UnsafeStrandCell<T> {
        UnsafeStrandCell {
            data: data
        }
    }

    pub unsafe fn get(&self) -> &mut T {
        &mut *(&self.data as *const _ as *mut _)
    }
}

unsafe impl<T> Send for UnsafeStrandCell<T> {}

unsafe impl<T> Sync for UnsafeStrandCell<T> {}
