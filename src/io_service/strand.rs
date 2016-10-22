use std::boxed::FnBox;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use error::ErrCode;
use unsafe_cell::UnsafeStrandCell;
use super::{IoObject, IoService, Callback, Handler, NoAsyncResult};

type Function<T> = Box<FnBox(*const IoService, *const StrandData<T>) + Send + 'static>;

struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<Function<T>>,
}

pub struct StrandData<T> {
    mutex: Arc<(Mutex<StrandQueue<T>>, UnsafeStrandCell<T>)>,
}

impl<T> StrandData<T> {
    pub fn dispatch<F>(&self, io: &IoService, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        {
            let mut owner = self.mutex.0.lock().unwrap();
            if owner.locked {
                owner.queue.push_back(Box::new(move |io: *const IoService, data: *const StrandData<T>| {
                    func(Strand { io: unsafe { &*io }, data: unsafe { &*data } });
                }));
                return;
            }
            owner.locked = true;
        }

        func(Strand { io: io, data: self });

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

    pub fn is_ownered(&self) -> bool {
        let owner = self.mutex.0.lock().unwrap();
        owner.locked
    }
}

impl<T> Clone for StrandData<T> {
    fn clone(&self) -> Self {
        StrandData {
            mutex: self.mutex.clone()
        }
    }
}

/// Provides serialized data and handler execution.
pub struct Strand<'a, T: 'a> {
    io: &'a IoService,
    data: &'a StrandData<T>,
}

impl<'a, T> Strand<'a, T> {
    /// Request the strand to invoke the given handler.
    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        func(Strand { io: self.io, data: self.data })
    }

    /// Returns a `&mut T` to the memory safely.
    pub fn get(&self) -> &mut T {
        unsafe { self.data.mutex.1.get() }
    }

    /// Request the strand to invoke the given handler and return immediately.
    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let mut owner = self.data.mutex.0.lock().unwrap();
        owner.queue.push_back(Box::new(move |io: *const IoService, data: *const StrandData<T>| func(Strand { io: unsafe { &*io }, data: unsafe { &*data } })))
    }

    /// Provides a `Strand` handler to asynchronous operation.
    ///
    /// The StrandHandler has trait the `Handler`, that type of `Handler::Output` is `()`.
    pub fn wrap<F, R, E>(&self, handler: F) -> StrandHandler<T, F, R, E>
        where F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
              R: Send + 'static,
    {
        StrandHandler {
            data: self.data.clone(),
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
        unsafe { self.data.mutex.1.get() }
    }
}

impl<'a, T> DerefMut for Strand<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.data.mutex.1.get() }
    }
}

/// Provides immutable data and handler execution.
pub struct StrandImmutable<'a, T> {
    io: &'a IoService,
    data: StrandData<T>,
}

impl<'a, T: 'static> StrandImmutable<'a, T> {
    /// Request the strand to invoke the given handler.
    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let data = self.data.clone();
        self.io.dispatch(move |io| data.dispatch(io, func))
    }

    /// Request the strand to invoke the given handler and return immediately.
    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let data = self.data.clone();
        self.io.post(move |io| data.dispatch(io, func))
    }

    /// Provides a `Strand` handler to asynchronous operation.
    ///
    /// The StrandHandler has trait the `Handler`, that type of `Handler::Output` is `()`.
    pub fn wrap<F, R, E>(&self, handler: F) -> StrandHandler<T, F, R, E>
        where F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
              R: Send + 'static,
    {
        StrandHandler {
            data: self.data.clone(),
            handler: handler,
            _marker: PhantomData,
        }
    }

    pub unsafe fn as_mut(&'a self) -> Strand<'a, T> {
        Strand {
            io: self.io,
            data: &self.data,
        }
    }
}

unsafe impl<'a, T> IoObject for StrandImmutable<'a, T> {
    fn io_service(&self) -> &IoService {
        self.io
    }
}

impl<'a, T> Deref for StrandImmutable<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.data.mutex.1.get() }
    }
}

/// The binding Strand handler.
pub struct StrandHandler<T, F, R, E> {
    pub data: StrandData<T>,
    handler: F,
    _marker: PhantomData<(R, E)>,
}

impl<T, F, R, E> Handler<R, E> for StrandHandler<T, F, R, E>
    where T: Send + 'static,
          F: FnOnce(Strand<T>, Result<R, E>) + Send + 'static,
          R: Send + 'static,
          E: Send + 'static,
{
    type Output = ();

    fn callback(self, io: &IoService, res: Result<R, E>) {
        let StrandHandler { data, handler, _marker } = self;
        handler(Strand { io: io, data: &data }, res)
    }

    fn wrap<G>(self, callback: G) -> Callback
        where G: FnOnce(&IoService, ErrCode, Self) + Send + 'static
    {
        Box::new(move |io: *const IoService, ec| {
            let io = unsafe { &*io };
            let StrandHandler { data, handler, _marker } = self;
            data.dispatch(io, move |st| {
                let Strand { io, data } = st;
                debug_assert_eq!(data.is_ownered(), true);
                callback(io, ec, StrandHandler {
                    data: data.clone(),
                    handler: handler,
                    _marker: _marker,
                })
            })
        })
    }

    type AsyncResult = NoAsyncResult;

    fn async_result(&self) -> Self::AsyncResult {
        NoAsyncResult
    }
}

pub fn strand_clone<'a, T>(io: &'a IoService, data: &'a StrandData<T>) -> Strand<'a, T> {
    Strand { io: io, data: data }
}

pub fn strand_new<'a, T>(io: &'a IoService, data: T) -> StrandImmutable<'a, T> {
    StrandImmutable {
        io: io,
        data: StrandData {
            mutex: Arc::new((Mutex::new(StrandQueue {
                locked: false,
                queue: VecDeque::new(),
            }), UnsafeStrandCell::new(data)))
        },
    }
}

#[test]
fn test_strand() {
    let io = &IoService::new();
    let st = IoService::strand(io, 0);
    let mut st = unsafe { st.as_mut() };
    *st = 1;
    assert_eq!(*st, 1);
}

#[test]
fn test_strand_dispatch() {
    let io = &IoService::new();
    let st = IoService::strand(io, 0);
    st.dispatch(|mut st| *st = 1);
    io.run();
    assert_eq!(*st, 1);
}

#[test]
fn test_strand_post() {
    let io = &IoService::new();
    let st = IoService::strand(io, 0);
    st.post(|mut st| *st = 1);
    io.run();
    assert_eq!(*st, 1);
}
