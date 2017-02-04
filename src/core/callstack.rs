use ffi::TssPtr;
use core::{IoContext, ThreadIoContext};

use std::ptr;
use std::ops::{Deref, DerefMut};

pub struct ThreadCallStack {
    key: *const IoContext,
    next: *mut ThreadCallStack,
    value: ThreadIoContext,
}

impl ThreadCallStack {
    pub fn new(value: ThreadIoContext) -> ThreadCallStack {
        ThreadCallStack {
            key: ptr::null(),
            next: ptr::null_mut(),
            value: value,
        }
    }

    pub fn wind(&mut self, key: &IoContext) -> ThreadCallStackRef {
        self.key = key;
        self.next = TOP.get();
        TOP.set(self);
        ThreadCallStackRef(self)
    }

    pub fn contains<'a>(key: &'a IoContext) -> Option<&'a mut ThreadIoContext> {
        let mut ptr = TOP.get();
        unsafe {
            while !ptr.is_null() {
                if (*ptr).key == key as *const IoContext {
                    return Some(&mut (*ptr).value);
                }
                ptr = (*ptr).next;
            }
        }
        None
    }
}

pub struct ThreadCallStackRef<'a>(&'a mut ThreadCallStack);

impl<'a> Drop for ThreadCallStackRef<'a> {
    fn drop(&mut self) {
        TOP.set(self.0.next)
    }
}

impl<'a> Deref for ThreadCallStackRef<'a> {
    type Target = ThreadIoContext;

    fn deref(&self) -> &Self::Target {
        &self.0.value
    }
}

impl<'a> DerefMut for ThreadCallStackRef<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.value
    }
}

lazy_static! {
    static ref TOP: TssPtr<ThreadCallStack> = TssPtr::new().unwrap();
}

#[test]
fn test_call_stack_1() {
    let ctx = IoContext::new().unwrap();
    let mut cs = ThreadCallStack::new(Default::default());
    {
        let _ctx = cs.wind(&ctx);
        assert!(ThreadCallStack::contains(&ctx).is_some());
    }
    assert!(ThreadCallStack::contains(&ctx).is_none());
}

#[test]
fn test_call_stack_2() {
    use std::thread;

    let ctx = IoContext::new().unwrap();
    let mut cs = ThreadCallStack::new(Default::default());
    let _ctx = cs.wind(&ctx);
    assert!(ThreadCallStack::contains(&ctx).is_some());
    {
        let ctx = ctx.clone();
        thread::spawn(move || {
            assert!(ThreadCallStack::contains(&ctx).is_none());
        }).join().unwrap();
    }
}
