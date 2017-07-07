use std::fmt;
use std::ops;
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};

pub struct PairBox<T>(*mut (T, AtomicBool));

impl<T> PairBox<T> {
    pub fn new(t: T) -> (PairBox<T>, PairBox<T>) {
        let ptr = Box::into_raw(Box::new((t, AtomicBool::new(false))));
        (PairBox(ptr), PairBox(ptr))
    }

    pub fn is_pair(&self, other: &PairBox<T>) -> bool {
        debug_assert_eq!(self as *const _, other as *const _);
        self.0 == other.0
    }

    pub fn has_pair(&self) -> bool {
        unsafe { &*self.0 }.1.load(Ordering::Relaxed)
    }
}

impl<T> Drop for PairBox<T> {
    fn drop(&mut self) {
        unsafe {
            if (&*self.0).1.swap(true, Ordering::SeqCst) {
                Box::from_raw(self.0);
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for PairBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe { &*self.0 }.fmt(f)
    }
}

impl<T> ops::Deref for PairBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &unsafe { &*self.0 }.0
    }
}

impl<T> ops::DerefMut for PairBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut unsafe { &mut *self.0 }.0
    }
}

unsafe impl<T: Send> Send for PairBox<T> {
}

#[test]
fn test() {
    let (a, b) = PairBox::new(0);
    assert_eq!(*a, 0);
    assert_eq!(*b, 0);

    let (a, b) = (a, b).replace(|x| *x = 1).unwrap();
    assert_eq!(*a, 1);
    assert_eq!(*b, 1);

    let (c, d) = PairBox::new(1);
    assert!( (a, c).replace(|x| *x = 2).is_err() );
}

#[test]
fn test_drop() {
    static mut EXIT_COUNT: usize = 0;

    struct Exit;
    impl Drop for Exit {
        fn drop(&mut self) {
            unsafe { EXIT_COUNT += 1; }
        }
    }

    {
        let (a, b) = PairBox::new(Exit);
        assert_eq!(unsafe { EXIT_COUNT }, 0);
    }
    assert_eq!(unsafe { EXIT_COUNT }, 1);
}

#[test]
fn test_thread() {
    use std::thread;

    let (a, b) = PairBox::new(0);
    let thrd = thread::spawn(move || {
        assert_eq!(*a, 0);
    });
    assert_eq!(*b, 0);
    thrd.join();
}
