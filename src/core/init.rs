use ffi::{startup, cleanup};

use std::sync::Mutex;

#[derive(Debug)]
pub struct Init;

impl Drop for Init {
    fn drop(&mut self) {
        let mut i = REGISTRY_COUNT.lock().unwrap();
        *i -= 1;
        if *i == 0 {
            cleanup();
        }
    }
}

impl Init {
    pub fn registry() -> Self {
        let mut i = REGISTRY_COUNT.lock().unwrap();
        *i += 1;
        if *i == 1 {
            startup();
        }
        Init
    }
}

lazy_static! {
    static ref REGISTRY_COUNT: Mutex<usize> = Default::default();
}
