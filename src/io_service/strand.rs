use std::io;
use std::boxed::FnBox;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use unsafe_cell::UnsafeStrandCell;
use super::{IoObject, IoService, Handler, NoAsyncResult};

type Function<T> = Box<FnBox(*const IoService, *const StrandImpl<T>) + Send + 'static>;

struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<Function<T>>,
}

pub struct StrandImpl<T> {
    mutex: Arc<(Mutex<StrandQueue<T>>, UnsafeStrandCell<T>)>,
}

impl<T> StrandImpl<T> {
    pub fn new(data: T, locked: bool) -> StrandImpl<T> {
        StrandImpl {
            mutex: Arc::new((Mutex::new(StrandQueue {
                locked: locked,
                queue: VecDeque::new(),
            }), UnsafeStrandCell::new(data)))
        }
    }

    pub fn do_dispatch(&self, io: &IoService) {
        while let Some(func) = {
            let mut owner = self.mutex.0.lock().unwrap();
            if let Some(func) = owner.queue.pop_front() {
                Some(func)
            } else {
                owner.locked = false;
                None
            }
        } {
            func(io, self);
        }
    }

    fn dispatch<F>(&self, func: F, io: &IoService)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let _ = {
            let mut owner = self.mutex.0.lock().unwrap();
            if owner.locked {
                owner.queue.push_back(Box::new(move |io: *const IoService, imp: *const StrandImpl<T>| {
                    func(Strand { io: unsafe { &*io }, imp: unsafe { &*imp } });
                }));
                return;
            }
        };

        func(Strand { io: io, imp: self });
        self.do_dispatch(io);
    }
}

impl<T> Clone for StrandImpl<T> {
    fn clone(&self) -> Self {
        StrandImpl {
            mutex: self.mutex.clone()
        }
    }
}

pub struct Strand<'a, T: 'a> {
    io: &'a IoService,
    imp: &'a StrandImpl<T>,
}

impl<'a, T: 'static> Strand<'a, T> {
    /// Request the strand to invoke the given handler.
    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        if self.io.0.running_in_this_thread() {
            func(Strand { io: self.io, imp: self.imp })
        } else {
            let imp = self.imp.clone();
            self.io.dispatch(move |io| {
                func(Strand { io: io, imp: &imp });
            })
        }
    }

    /// Returns a `&mut T` to the memory safely.
    pub fn get(&self) -> &mut T {
        unsafe { self.imp.mutex.1.get() }
    }

    /// Request the strand to invoke the given handler and return immediately.
    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        if self.io.0.running_in_this_thread() {
            let mut owner = self.imp.mutex.0.lock().unwrap();
            owner.queue.push_back(Box::new(move |io: *const IoService, imp: *const StrandImpl<T>| {
                func(Strand { io: unsafe { &*io }, imp: unsafe { &*imp } });
            }));
        } else {
            let imp = self.imp.clone();
            self.io.post(move |io| {
                func(Strand { io: io, imp: &imp });
            })
        }
    }

    /// Returns a `Strand` handler to asynchronous operation.
    pub fn wrap<F, R>(&self, handler: F) -> StrandHandler<T, F, R>
        where F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
              R: Send + 'static,
    {
        StrandHandler {
            imp: self.imp.clone(),
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<'a, T> IoObject for Strand<'a, T> {
    fn io_service(&self) -> &IoService {
        self.io
    }
}

impl<'a, T> Deref for Strand<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.imp.mutex.1.get() }
    }
}

impl<'a, T> DerefMut for Strand<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.imp.mutex.1.get() }
    }
}

/// The binding Strand handler.
pub struct StrandHandler<T, F, R> {
    pub imp: StrandImpl<T>,
    handler: F,
    _marker: PhantomData<R>,
}

impl<T, F, R> Handler<R> for StrandHandler<T, F, R>
    where T: Send + 'static,
          F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static
{
    type Output = ();

    fn callback(self, io: &IoService, res: io::Result<R>) {
        let StrandHandler { imp, handler, _marker } = self;
        imp.dispatch(move |io| handler(io, res), io);
    }

    #[doc(hidden)]
    type AsyncResult = NoAsyncResult;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult {
        NoAsyncResult
    }
}

pub fn strand<'a, T>(io: &'a IoService, imp: &'a StrandImpl<T>) -> Strand<'a, T> {
    Strand { io: io, imp: imp }
}

#[test]
fn test_strand() {
    let io = &IoService::new();
    IoService::strand(io, 0, |mut st| {
        *st = 1;
        assert_eq!(*st, 1);
    });
}

#[test]
fn test_strand_dispatch() {
    let io = &IoService::new();
    IoService::strand(io, 0, |st| {
        st.dispatch(|mut st| *st = 1);
        st.io_service().run();
        assert_eq!(*st, 1);
    });
}

#[test]
fn test_strand_post() {
    let io = &IoService::new();
    IoService::strand(io, 0, |st| {
        st.post(|mut st| *st = 1);
        io.run();
        assert_eq!(*st, 1);
    });
}

#[test]
fn test_strand_guard_dispatch() {
    let io = &IoService::new();
    IoService::strand(io, 0, |st| {
        st.post(|st| {
            st.dispatch(|mut st| *st = 1);
            assert_eq!(*st, 1);
        });
        io.run();
    });
}

#[test]
fn test_strand_guard_post() {
    let io = &IoService::new();
    IoService::strand(io, 0, |st| {
        st.post(|st| st.post(|mut st| {
            *st = 1;
            assert_eq!(*st, 1);
        }));
        io.run();
        assert_eq!(*st, 0);
    });
}
