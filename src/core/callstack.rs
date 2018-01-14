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
        debug_assert!(self.next.is_null());
        self.next = TOP.get() as *mut _;
        TOP.set(self as *mut _ as *mut _);
    }

    pub fn callstack<'a>(key: &'a K) -> Option<&'a mut Self> {
        let mut ptr = TOP.get() as *mut Self;
        unsafe {
            while !ptr.is_null() {
                if key.eq(&*(*ptr).key) {
                    return Some(&mut *ptr);
                }
                ptr = (*ptr).next;
            }
        }
        None
    }
}

impl<K, V> Drop for ThreadCallStack<K, V> {
    fn drop(&mut self) {
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

use super::{AsIoContext, IoContext};

unsafe impl<K: AsIoContext, V> AsIoContext for ThreadCallStack<K, V> {
    fn as_ctx(&self) -> &IoContext {
        unsafe { &*self.key }.as_ctx()
    }
}

#[test]
fn test_call_stack_1() {
    type ThreadIoContext = ThreadCallStack<IoContext, i32>;

    let ctx = &IoContext::new().unwrap();
    {
        let mut thread = ThreadIoContext::new(ctx, 0);
        thread.init();
        assert!(ThreadIoContext::callstack(ctx).is_some());
    }
    assert!(ThreadIoContext::callstack(ctx).is_none());
}

#[test]
fn test_call_stack_2() {
    use std::thread;
    type ThreadIoContext = ThreadCallStack<IoContext, i32>;

    let ctx = &IoContext::new().unwrap();
    let mut thread = ThreadIoContext::new(ctx, 0);
    thread.init();
    assert!(ThreadIoContext::callstack(ctx).is_some());

    let ctx = ctx.clone();
    thread::spawn(move || {
        assert!(ThreadIoContext::callstack(&ctx).is_none());
    }).join()
        .unwrap();
}

#[test]
fn test_callstack_3() {
    type ThreadIoContext = ThreadCallStack<IoContext, i32>;

    let ctx1 = &IoContext::new().unwrap();
    let mut thread1 = ThreadIoContext::new(ctx1, 0);
    thread1.init();

    let ctx2 = &IoContext::new().unwrap();
    assert!(ThreadIoContext::callstack(ctx1).is_some());
    assert!(ThreadIoContext::callstack(ctx2).is_none());

    let mut thread2 = ThreadIoContext::new(ctx2, 0);
    thread2.init();
    assert!(ThreadIoContext::callstack(ctx1).is_some());
    assert!(ThreadIoContext::callstack(ctx2).is_some());
}
