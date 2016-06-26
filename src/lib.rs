// Copyright 2016 Haruhiko Uchida
// The software is released under the MIT license.
// http://opensource.org/licenses/mit-license.php

//! The asio is Asynchronous Input/Output library.
//!
//!
//! # Usage
//!
//! This crate is on [github](https://github.com/harre-orz/rust_asio.git "github") and can be used by adding `asio` to the dependencies in your project's Cargo.toml.
//!
//! ```toml
//! [dependencies]
//! rust_asio = "*"
//! ```
//!
//! And this in your crate root:
//!
//! ```
//! extern crate asio;
//! ```
//!
//! For example, Connection with TCP socket code:
//!
//! ```
//! use std::io;
//! use asio::*;
//! use asio::ip::*;
//!
//! struct TcpClient(TcpSocket);
//!
//! impl TcpClient {
//!   fn start(io: &IoService) {
//!     let soc = Strand::new(io, TcpClient(TcpSocket::new(io, Tcp::v4()).unwrap()));
//!     let ep = TcpEndpoint::new((IpAddrV4::new(192,168,0,1), 12345));
//!     TcpSocket::async_connect(|soc| &soc.0, &ep, Self::on_connect, &soc);
//!   }
//!
//!   fn on_connect(soc: Strand<Self>, res: io::Result<()>) {
//!     match res {
//!       Ok(_) => println!("connected."),
//!       Err(err) => println!("{:?}", err),
//!     }
//!   }
//! }
//!
//! fn main() {
//!   let io = IoService::new();
//!   TcpClient::start(&io);
//!   //io.run();
//! }
//! ```

#![feature(test)]
#![feature(fnbox)]

extern crate test;
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

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Traits to the associated with `IoService`.
pub trait IoObject : Sized {
    /// Returns a `IoService` associated with this object.
    fn io_service(&self) -> &IoService;
}

/// The core I/O process.
///
/// This is a data for all of the process of referring an `IoService`.
///
/// # Examples
/// In this example, Set 3 closures and invoke all given closures at `run()`.
///
/// ```
/// use asio::IoService;
///
/// let io = IoService::new();
/// for i in 0..3 {
///     io.post(move |_| println!("do work {}", i+1));
/// }
/// io.run();
///
/// // --- Results ---
/// // do work 1
/// // do work 2
/// // do work 3
/// ```
///
/// In this example, Sets a closure in a nested closure.
///
/// ```
/// use asio::IoService;
///
/// let io = IoService::new();
/// io.post(move |io| {
///     io.post(move |_| println!("do work 2"));
///     println!("do work 1");
/// });
/// io.run();
///
/// // --- Results ---
/// // do work 1
/// // do work 2
/// ```
#[derive(Clone)]
pub struct IoService(Arc<Backbone>);

impl IoService {
    /// Make a new `IoService`.
    ///
    /// # Panics
    /// Panics if too many open file.
    ///
    /// # Examples
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// ```
    pub fn new() -> IoService {
        IoService(Arc::new(Backbone::new().unwrap()))
    }

    /// Set a stop request and cancel all of the waiting event in an `IoService`.
    ///
    /// # Examples
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// io.stop();
    /// ```
    pub fn stop(&self) {
        self.0.stop()
    }

    /// Determine whether a `IoService` has been stopped.
    ///
    /// # Examples
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// assert_eq!(io.stopped(), false);
    /// io.stop();
    /// assert_eq!(io.stopped(), true);
    /// ```
    pub fn stopped(&self) -> bool {
        self.0.task.stopped()
    }

    /// Reset a stopped `IoService`.
    ///
    /// # Examples
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// assert_eq!(io.stopped(), false);
    /// io.stop();
    /// assert_eq!(io.stopped(), true);
    /// io.reset();
    /// assert_eq!(io.stopped(), false);
    /// ```
    pub fn reset(&self) {
        self.0.task.reset()
    }

    /// Request a process to invoke the given handler and return immediately.
    ///
    /// # Examples
    /// ```
    /// use asio::IoService;
    /// use std::sync::atomic::*;
    ///
    /// let io = IoService::new();
    /// static PASS: AtomicBool = ATOMIC_BOOL_INIT;
    ///
    /// io.post(|_| PASS.store(true, Ordering::Relaxed));
    /// assert_eq!(PASS.load(Ordering::Relaxed), false);
    ///
    /// io.run();
    /// assert_eq!(PASS.load(Ordering::Relaxed), true);
    /// ```
    pub fn post<F>(&self, callback: F)
        where F: FnOnce(&IoService) + Send + 'static {
        self.0.post(0, Box::new(move |io: *const IoService| callback(unsafe { &*io })));
    }

    /// Request a process to invoke the given handler with serialized by `Strand` and return immediately.
    ///
    /// # Examples
    /// ```
    /// use asio::{IoService, Strand};
    ///
    /// let io = IoService::new();
    /// let pass = Strand::new(&io, false);
    ///
    /// io.post_strand(|mut pass| *pass = true, &pass);
    /// assert_eq!(*pass, false);
    ///
    /// io.run();
    /// assert_eq!(*pass, true);
    /// ```
    pub fn post_strand<'a, F, T>(&self, callback: F, strand: &Strand<'a, T>)
        where F: FnOnce(Strand<'a, T>) + Send + 'static,
              T: 'static {
        let obj = strand.obj.clone();
        self.0.post(strand.id(), Box::new(
            move |io: *const IoService| callback(Strand { io: unsafe { &*io }, obj: obj }))
        );
    }

    /// Run all given handlers.
    ///
    /// # Examples
    /// ```
    /// use asio::IoService;
    ///
    /// let io = IoService::new();
    /// io.run();
    /// ```
    pub fn run(&self) {
        if !self.stopped() {
            Backbone::run(self);
        }
    }

    /// Run all given handlers until call the `stop()`.
    ///
    /// This is ensured to not exit until explicity stopped, so it can invoking given handlers in multi-threads.
    ///
    /// # Examples
    /// Execute 5 parallel's event loop (4 thread::spawn + 1 main thread).
    ///
    /// ```
    /// use asio::IoService;
    /// use std::thread;
    ///
    /// let mut thrds = Vec::new();
    /// IoService::new().work(|io| {
    ///     for _ in 0..4 {
    ///         let io = io.clone();
    ///         thrds.push(thread::spawn(move || io.run()));
    ///     }
    ///
    ///     io.post(move |io| {
    ///         io.stop();  // If does not explicity stop, not returns in this `work()`.
    ///     });
    /// });
    ///
    /// for thrd in thrds {
    ///     thrd.join().unwrap();
    /// }
    /// ```
    pub fn work<F: FnOnce(&IoService)>(&self, callback: F) {
        if !self.stopped() {
            self.0.task.set_work(true);
            callback(self);
            Backbone::run(self);
            self.0.task.set_work(false);
        }
    }
}

impl IoObject for IoService {
    fn io_service(&self) -> &IoService {
        self
    }
}

impl PartialEq for IoService {
    fn eq(&self, other: &Self) -> bool {
        (&*self.0 as *const Backbone) == (&*other.0 as *const Backbone)
    }
}

impl Eq for IoService {
}

impl fmt::Debug for IoService {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IoService")
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

impl<T> Deref for UnsafeThreadableCell<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for UnsafeThreadableCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

unsafe impl<T> Send for UnsafeThreadableCell<T> {}

unsafe impl<T> Sync for UnsafeThreadableCell<T> {}

/// Serialized object for an `IoService`.
///
/// This is cannot `Send` and `Sync`, but possible to move another thread in event loop.
///
/// # Examples
/// ```
/// use asio::{IoObject, IoService, Strand};
/// use std::thread;
///
/// let mut thrds = Vec::new();
/// IoService::new().work(|io| {
///     for _ in 0..4 {
///         let io = io.clone();
///         thrds.push(thread::spawn(move || io.run()));
///     }
///
///     fn closure(mut counter: Strand<usize>) {
///         if *counter != 100 {
///             *counter += 1;
///             counter.io_service().post_strand(closure, &counter);
///         }
///     }
///     for _ in 0..10 {
///         closure(Strand::new(io, 0));
///     }
///
///     io.stop();
/// });
///
/// for thrd in thrds {
///     thrd.join().unwrap();
/// }
/// ```
pub struct Strand<'a, T> {
    io: &'a IoService,
    obj: Arc<UnsafeThreadableCell<T>>,
}

impl<'a, T> Strand<'a, T> {
    /// Make a `Strand` wrapped value.
    ///
    /// # Examples
    /// ```
    /// use asio::{IoService, Strand};
    ///
    /// let io = IoService::new();
    /// let obj = Strand::new(&io, false);
    /// assert_eq!(*obj, false);
    /// ```
    pub fn new(io: &'a IoService, t: T) -> Strand<'a, T> {
        Strand {
            io: io,
            obj: Arc::new(UnsafeThreadableCell::new(t)),
        }
    }

    fn id(&self) -> usize {
        unsafe { self.obj.get() as usize }
    }

    fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.obj.get() }
    }
}

impl<'a, T> IoObject for Strand<'a, T> {
    fn io_service(&self) -> &IoService {
        self.io
    }
}

impl<'a, T> Deref for Strand<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.obj.get() }
    }
}

impl<'a, T> DerefMut for Strand<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.obj.get() }
    }
}

pub trait Cancel {
    fn cancel(&self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::*;
    use std::thread;

    #[test]
    fn test_io_service() {
        let io = IoService::new();
        io.stop();
        io.run();
    }

    #[test]
    fn test_io_run() {
        static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

        let io = IoService::new();
        for _ in 0..10 {
            io.post(|_| { COUNT.fetch_add(1, Ordering::Relaxed); });
        }
        assert!(COUNT.load(Ordering::Relaxed) == 0);

        io.run();
        assert!(COUNT.load(Ordering::Relaxed) == 10);
    }

    #[test]
    fn test_io_stop_and_reset() {
        static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

        let io = IoService::new();
        for _ in 0..10 {
            let io_ = io.clone();
            io.post(|_| { COUNT.fetch_add(1, Ordering::Relaxed); });
        }
        io.stop();
        io.run();
        assert!(COUNT.load(Ordering::Relaxed) == 0);
        io.reset();
        io.run();
        assert!(COUNT.load(Ordering::Relaxed) == 10);
    }

    #[test]
    fn test_io_multi_thread() {
        IoService::new().work(|io| {
            static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
            let mut thrds = Vec::new();
            for _ in 0..5 {
                let io = io.clone();
                thrds.push(thread::spawn(move || io.run()));
            }

            for _ in 0..1000 {
                io.post(|io| if COUNT.fetch_add(1, Ordering::Relaxed) == 999 {
                    io.stop();
                });
            }

            for thrd in thrds {
                thrd.join().unwrap();
            }
            assert!(COUNT.load(Ordering::Relaxed) == 1000);
        });
    }

    #[test]
    fn test_strand_id() {
        let io = IoService::new();
        let strand = Strand::new(&io, 0);
        assert!(strand.id() == (Strand { io: &io, obj: strand.obj.clone() }).id());
    }
}
