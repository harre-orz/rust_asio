// Copyright 2016 Haruhiko Uchida
// The software is released under the MIT license.
// http://opensource.org/licenses/mit-license.php

//! asio is ASynchronous Input/Output library like boost::asio.
//!
//! # Usage
//!
//! This crate is on [github](https://github.com/harre-orz/rust_asio.git "github") and can be used by adding `asio` to the dependencies in your project's Cargo.toml.
//!
//! ```toml
//! [dependencies]
//! asio = "0.1"
//! ```
//!
//! And this in your crate root:
//!
//! ```
//! extern crate asio;
//! ```
//! For more read [README](https://github.com/harre-orz/rust_asio/blob/master/README.md "README").

#![feature(fnbox)]
#![feature(optin_builtin_traits)]

extern crate libc;
extern crate time;

#[cfg(feature = "developer")] pub mod ops;
#[cfg(feature = "developer")] pub mod backbone;
#[cfg(not(feature = "developer"))] mod ops;
#[cfg(not(feature = "developer"))] mod backbone;
use backbone::Backbone;

mod socket;
pub use self::socket::*;
mod timer;
pub use self::timer::*;
mod str;

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Provides I/O object.
pub trait IoObject : Sized {
    /// Return the `IoService` associated with the object.
    fn io_service(&self) -> &IoService;
}

/// Provides I/O process.
///
/// This is an asynchronus based object. All of the `IoObject` are associated from `IoService` object.
#[derive(Clone)]
pub struct IoService(Arc<Backbone>);

impl IoService {
    /// Make the new `IoService` object.
    pub fn new() -> IoService {
        IoService(Arc::new(Backbone::new().unwrap()))
    }

    /// Determine whether the `IoService` has been stopped.
    pub fn stopped(&self) -> bool {
        self.0.task.stopped()
    }

    /// Stop the `IoService` object's event processing loop.
    pub fn stop(&self) {
        self.0.stop()
    }

    /// Reset the stopped `IoService` object's.
    ///
    /// # Example
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// static mut worked: bool = false;
    /// io.post(|| unsafe { worked = true });
    ///
    /// io.stop();
    /// io.run();
    /// assert!(unsafe { !worked });
    ///
    /// io.reset();
    /// io.run();
    /// assert!(unsafe { worked });
    /// ```
    pub fn reset(&self) {
        self.0.task.reset()
    }

    /// Request the process to invoke the given handler and return immediately.
    ///
    /// # Example
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// io.post(|| { panic!("do not work") });
    /// ```
    pub fn post<F>(&self, callback: F)
        where F: FnOnce() + Send + 'static {
        self.0.task.post(0, Box::new(callback))
    }

    pub fn post_strand<F, T>(&self, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>) + Send + 'static,
              T: 'static {
        let arc = strand.0.clone();
        self.0.task.post(strand.id(), Box::new(move || callback(Strand(arc))));
    }

    /// Run all given handlers.
    ///
    /// # Example
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// io.post(|| { println!("do work") });
    /// assert!(io.run() == 1);
    /// ```
    pub fn run(&self) -> usize {
        self.0.task.run()
    }

    /// Run a first given handler.
    ///
    /// # Example
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// io.post(|| { println!("do work") });
    /// io.post(|| { println!("do not work") });
    /// assert!(io.run_one() == 1);
    /// ```
    pub fn run_one(&self) -> usize {
        self.0.task.run_one()
    }

    /// Return the `IoServiceWork` object.
    ///
    /// # Example
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// {
    ///     let work = io.work();
    ///     io.stop();
    /// }
    /// assert!(io.stopped());
    /// ```
    pub fn work<'a>(&'a self) -> IoServiceWork<'a> {
        self.0.task.block();
        IoServiceWork(self)
    }
}

impl IoObject for IoService {
    fn io_service(&self) -> &IoService {
        self
    }
}

/// Multiple I/O thread work.
///
/// The wor ensures that
/// The work ensures  will not exit until `IoService::stop()`, and that it does exit when there is no unfinished work remaining.
/// The `Drop` with notifies stop the `IoService` that the work is complete.
pub struct IoServiceWork<'a>(&'a IoService);

impl<'a> IoObject for IoServiceWork<'a> {
    fn io_service(&self) -> &IoService {
        self.0.io_service()
    }
}

impl<'a> Drop for IoServiceWork<'a> {
    fn drop(&mut self) {
        (self.0).0.task.run();
        (self.0).0.task.clear();
    }
}

struct UnsafeThreadableCell<T> {
    value: T,
}

impl<T> UnsafeThreadableCell<T> {
    fn new(value: T) -> UnsafeThreadableCell<T> {
        UnsafeThreadableCell {
            value: value,
        }
    }

    unsafe fn get(&self) -> *mut T {
        &self.value as *const T as *mut T
    }
}

unsafe impl<T> Send for UnsafeThreadableCell<T> {}

unsafe impl<T> Sync for UnsafeThreadableCell<T> {}

/// Serialized object.
pub struct Strand<T>(Arc<(IoService, UnsafeThreadableCell<T>)>);

impl<T> Strand<T> {
    // Make the `Strand` wrapped object.
    pub fn new(io: &IoService, t: T) -> Strand<T> {
        Strand(Arc::new((io.clone(), UnsafeThreadableCell::new(t))))
    }

    fn id(&self) -> usize {
        unsafe { (*self.0).1.get() as usize }
    }

    fn get_mut(&self) -> &mut T {
        unsafe { &mut *(*self.0).1.get() }
    }
}

impl<T> IoObject for Strand<T> {
    fn io_service(&self) -> &IoService {
        &(self.0).0
    }
}

impl<T> Deref for Strand<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(*self.0).1.get() }
    }
}

impl<T> DerefMut for Strand<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(*self.0).1.get() }
    }
}

impl<T> !Send for Strand<T> {}

impl<T> !Sync for Strand<T> {}

pub trait Cancel {
    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self;
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
            thrd.join().unwrap();
        }
    }
}


#[test]
fn test_io_service_work() {
    use std::thread;

    let io = IoService::new();
    let mut thrds = Vec::new();
    for _ in 0..10 {
        let io = io.clone();
        thrds.push(thread::spawn(move || io.run()));
    }
    static mut stopped: bool = false;
    {
        let work = io.work();
        for i in 0..1000 {
            let _io = io.clone();
            io.post(move || {
                if i == 999 {
                    _io.stop();
                    unsafe { stopped = true };
                }
            });
        }
    }
    assert!(unsafe { stopped });
    for thrd in thrds {
        thrd.join();
    }
}

#[test]
fn test_strand_id() {
    let io = IoService::new();
    let strand = Strand::new(&io, 0);
    assert!(strand.id() == Strand(strand.0.clone()).id());
}
