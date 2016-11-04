use std::sync::RwLock;
use std::sync::atomic::{ATOMIC_USIZE_INIT, Ordering, AtomicUsize};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use thread_id;
use super::Callback;

lazy_static! {
    static ref CALL_STACK: CallStack = CallStack::new();
}

static UID: AtomicUsize = ATOMIC_USIZE_INIT;

#[derive(Default)]
pub struct ThreadInfoImpl {
}

unsafe impl Send for ThreadInfoImpl {
}

unsafe impl Sync for ThreadInfoImpl {
}

pub struct CallStack {
    pool: RwLock<HashMap<usize, ThreadInfoImpl>>,
}

impl CallStack {
    fn new() -> CallStack {
        CallStack {
            pool: RwLock::new(HashMap::new()),
        }
    }

    pub fn contains() -> bool {
        let tid = thread_id::get();
        let pool = CALL_STACK.pool.read().unwrap();
        pool.contains_key(&tid)
    }
}

pub struct ThreadInfo {
    uid: usize,
    info: *mut ThreadInfoImpl,
}

impl ThreadInfo {
    pub fn new() -> Option<ThreadInfo> {
        let tid = thread_id::get();
        let mut pool = CALL_STACK.pool.write().unwrap();
        if pool.contains_key(&tid) {
            None
        } else {
            Some(ThreadInfo {
                uid: UID.fetch_add(1, Ordering::SeqCst),
                info: pool.entry(tid).or_insert(Default::default()),
            })
        }
    }

    #[allow(dead_code)]
    pub fn id(&self) -> usize {
        self.uid
    }
}

impl Drop for ThreadInfo {
    fn drop(&mut self) {
        let tid= thread_id::get();
        let mut pool = CALL_STACK.pool.write().unwrap();
        pool.remove(&tid).unwrap();
    }
}

impl Deref for ThreadInfo {
    type Target = ThreadInfoImpl;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.info }
    }
}

impl DerefMut for ThreadInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.info }
    }
}

#[test]
fn test_thread_info() {
    assert_eq!(CallStack::contains(), false);
    let _ti = ThreadInfo::new().unwrap();
    assert_eq!(CallStack::contains(), true);

    use std::thread;
    thread::spawn(|| {
        assert_eq!(CallStack::contains(), false);
        let _ti = ThreadInfo::new().unwrap();
        assert_eq!(CallStack::contains(), true);
    }).join().unwrap();
}

#[test]
fn test_thread_info_dup() {
    let _ti = ThreadInfo::new().unwrap();
    assert!(ThreadInfo::new().is_none());
    assert_eq!(CallStack::contains(), true);
}
