#![feature(fnbox)]
extern crate libc;
extern crate time;

#[cfg(feature = "developer")]
pub mod ops;
#[cfg(not(feature = "developer"))]
mod ops;

#[cfg(feature = "developer")]
pub mod backbone;
#[cfg(not(feature = "developer"))]
mod backbone;

mod socket;
pub use self::socket::*;
mod timer;
pub use self::timer::*;
mod str;

use std::ops::{Deref, DerefMut};
use std::cell::UnsafeCell;
use std::sync::Arc;
use backbone::{Expiry, Backbone, TaskExecutor};

pub trait IoObject : Sized {
    fn io_service(&self) -> IoService;
}

#[derive(Clone)]
pub struct IoService(Arc<Backbone>);

impl IoService {
    pub fn new() -> IoService {
        IoService(Arc::new(Backbone::new().unwrap()))
    }

    pub fn stopped(&self) -> bool {
        TaskExecutor::stopped(self)
    }

    pub fn stop(&self) {
        Backbone::stop(self);
    }

    pub fn reset(&self) {
        TaskExecutor::reset(self);
    }

    pub fn post<F: FnOnce() + Send + 'static>(&self, callback: F) {
        TaskExecutor::post(self, Box::new(callback))
    }

    fn post_strand<F: FnOnce() + Send + 'static, T>(&self, callback: F, strand: &Strand<T>) {
        TaskExecutor::post_strand_id(self, Box::new(callback), strand.id())
    }

    pub fn run(&self) -> usize {
        TaskExecutor::run(self)
    }

    pub fn run_one(&self) -> usize {
        TaskExecutor::run_one(self)
    }

    fn interrupt(&self) {
        Backbone::interrupt(self);
    }

    fn timeout(&self, expiry: Expiry) {
        Backbone::timeout(self, expiry)
    }
}

impl IoObject for IoService {
    fn io_service(&self) -> IoService {
        self.clone()
    }
}

pub struct IoServiceWork<'a>(&'a IoService);

impl IoService {
    pub fn work<'a>(&'a self) -> IoServiceWork<'a> {
        TaskExecutor::block(self);
        IoServiceWork(self)
    }
}

impl<'a> IoObject for IoServiceWork<'a> {
    fn io_service(&self) -> IoService {
        self.0.io_service()
    }
}

impl<'a> Drop for IoServiceWork<'a> {
    fn drop(&mut self) {
        self.0.stop();
        TaskExecutor::clear(self.0);
    }
}

pub struct Strand<T>(Arc<UnsafeCell<T>>);

impl<T> Strand<T> {
    pub fn new(t: T) -> Strand<T> {
        Strand(Arc::new(UnsafeCell::new(t)))
    }

    fn id(&self) -> usize {
        (*self.0).get() as usize
    }

    fn get_mut(&self) -> &mut T {
        unsafe { &mut *(*self.0).get() }
    }
}

unsafe impl<T> Send for Strand<T> {}
unsafe impl<T> Sync for Strand<T> {}

impl<T> Clone for Strand<T> {
    fn clone(&self) -> Strand<T> {
        Strand(self.0.clone())
    }
}

impl<T> Deref for Strand<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(*self.0).get() }
    }
}

impl<T> DerefMut for Strand<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(*self.0).get() }
    }
}

#[test]
fn test_io_service() {
    let io = IoService::new();
    assert!(io.run() == 0);
}

#[test]
fn test_io_run_one() {
    static mut flag: bool = false;
    let io = IoService::new();
    io.post(|| unsafe { flag = true; });
    assert!(unsafe { flag == false });
    io.run_one();
    assert!(unsafe { flag == true });
}

#[test]
fn test_io_run_all() {
    static mut count: i32 = 0;
    let io = IoService::new();
    for _ in 0..10 {
        io.post(|| unsafe { count+= 1; });
    }
    assert!(unsafe { count == 0 });
    io.run();
    assert!(unsafe { count == 10});
}

#[test]
fn test_io_stop() {
    static mut count: i32 = 0;
    let io = IoService::new();
    for _ in 0..3 {
        let child = io.clone();
        io.post(move || { child.stop(); unsafe { count += 1; }});
    }
    assert!(unsafe { count == 0 });
    io.run();
    assert!(unsafe { count == 1 });
    io.run();
    assert!(unsafe { count == 1 });
}

#[test]
fn test_io_reset() {
    static mut count: i32 = 0;
    let io = IoService::new();
    for _ in 0..3 {
        let child = io.clone();
        io.post(move || { child.stop(); unsafe { count += 1; }});
    }
    assert!(unsafe { count == 0 });
    io.run();
    assert!(unsafe { count == 1 });
    io.reset();
    io.run();
    assert!(unsafe { count == 2 });
}

#[test]
fn test_io_block() {
    static mut count: i32 = 0;
    let io = IoService::new();
    for _ in 0..3 {
        let child = io.clone();
        io.post(move || { child.stop(); unsafe { count += 1; }});
    } {
        let work = io.work();
        assert!(unsafe { count == 0 });
    }
    assert!(unsafe { count == 3 });
}

#[test]
fn test_io_multi_thread() {
    use std::thread;
    use std::sync::{Arc, Mutex};

    let count: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let io = IoService::new();
    {
        let work = io.work();
        let mut thrds = Vec::new();
        for _ in 0..5 {
            let count = count.clone();
            let io = io.clone();
            thrds.push(thread::spawn(move || {
                io.run();
                let count = count.lock().unwrap();
                assert!(*count == 1000);
            }));
        }

        for _ in 0..1000 {
            let count = count.clone();
            let child = io.clone();
            io.post(move || {
                let mut count = count.lock().unwrap();
                assert!(*count < 1000);
                *count += 1;
                if *count == 1000 {
                    child.stop();
                }
            });
        }

        for thrd in thrds {
            thrd.join();
        }
    }
}

#[test]
fn test_strand_id() {
    let strand = Strand::new(100);
    assert!(strand.clone().id() == strand.id());
}
