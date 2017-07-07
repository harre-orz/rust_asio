use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct LazyBox<T> {
    data: Arc<(T, AtomicBool)>,
    ownered: bool,
}

impl<T> Drop for LazyBox<T> {
    fn drop(&mut self) {
        self.data.1.store(false, Ordering::SeqCst);
    }
}

impl<T> LazyBox<T> {
    fn new(t: T) -> Self {
        LazyBox {
            data: Arc::new((t, AtomicBool::new(true))),
            ownered: true,
        }
    }
}

impl<T> Clone for LazyBox<T> {
    fn clone(&self) -> Self {
        LazyBox {
            data: self.data.clone(),
            ownered: false
        }
    }
}

impl<T> Deref for LazyBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        if !self.ownered {
            if self.data.1.compare_and_swap(false, true, Ordering::SeqCst) == true {
                panic!("bad access");
            }
            unsafe { *(&self.ownered as *const _ as *mut _) = true; }
        }
        &self.data.0
    }
}

impl<T> DerefMut for LazyBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if !self.ownered {
            if self.data.1.compare_and_swap(false, true, Ordering::SeqCst) == true {
                panic!("bad access");
            }
            unsafe { *(&self.ownered as *const _ as *mut _) = true; }
        }
        unsafe { &mut *(&self.data.0 as *const _ as *mut _) }
    }
}

unsafe impl<T> Send for LazyBox<T>
    where T: Send {}

unsafe impl<T> Sync for LazyBox<T>
    where T: Sync {}

#[test]
fn test_new() {
    let mut a = LazyBox::new(0);  // lock a
    assert_eq!(*a, 0);
    *a = 1;
    assert_eq!(*a, 1);
}

#[test]
fn test_clone() {
    let mut a = LazyBox::new(0).clone();  // non-lock a
    {
        let mut b = a.clone();  // lock b
        *b = 1;
        assert_eq!(*b, 1);
    }
    *a = 2;  // lock a
    assert_eq!(*a, 2);
}

#[test]
#[should_panic]
fn test_failed_access() {
    let a = LazyBox::new(0);  // lock a
    let b = a.clone();
    assert_eq!(*b, 0);        // failed to lock b
}
