use std::sync::RwLock;
use thread_id;

pub struct CallStack {
    lock: RwLock<Vec<usize>>,
}

impl CallStack {
    pub fn new() -> CallStack {
        CallStack {
            lock: RwLock::new(Vec::new()),
        }
    }

    pub fn contains(&self) -> bool {
        let id = thread_id::get();
        let call = self.lock.read().unwrap();
        call.iter().find(|e| **e == id).is_some()
    }

    pub fn register(&self) {
        let id = thread_id::get();
        let mut call = self.lock.write().unwrap();
        call.push(id);
    }

    pub fn unregister(&self) {
        let id = thread_id::get();
        let mut call = self.lock.write().unwrap();
        let (idx, _) = call.iter().enumerate().find(|e| *e.1 == id).unwrap();
        call.remove(idx);
    }

    pub fn multi_threading(&self) -> bool {
        let call = self.lock.read().unwrap();
        call.len() > 1
    }
}
