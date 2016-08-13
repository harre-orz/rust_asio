use std::io;
use std::sync::{Arc, Mutex};
use std::boxed::FnBox;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use {IoObject, IoService, Handler};

type Value<T> = Arc<(UnsafeStrandCell<T>, Mutex<StrandQueue<T>>)>;

type TaskHandler<T> = Box<FnBox(*const IoService, *const Value<T>) + Send + 'static>;

struct UnsafeStrandCell<T> {
    data: T,
}

impl<T> UnsafeStrandCell<T> {
    unsafe fn get(&self) -> &mut T {
        &mut *(&self.data as *const _ as *mut _)
    }
}

unsafe impl<T> Send for UnsafeStrandCell<T> {}

unsafe impl<T> Sync for UnsafeStrandCell<T> {}

struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<TaskHandler<T>>,
}

impl<T> Default for StrandQueue<T> {
    fn default() -> StrandQueue<T> {
        StrandQueue {
            locked: false,
            queue: VecDeque::new(),
        }
    }
}

/// The binding Strand<T> handler.
pub struct StrandHandler<T, F, R> {
    value: Value<T>,
    handler: F,
    marker: PhantomData<R>,
}

impl<T, F, A, R> Handler<A, R> for StrandHandler<T, F, R>
    where T: Send + 'static,
          F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static,
{
    fn callback(self, io: &IoService, _: &A, res: io::Result<R>) {
        let StrandHandler { value, handler, marker:_ } = self;
        let _ = {
            let mut owner = value.1.lock().unwrap();
            if owner.locked {
                owner.queue.push_back(Box::new(move |io: *const IoService, value: *const Value<T>| {
                    let strand = Strand { io: unsafe { &*io }, value: unsafe { &*value }.clone() };
                    handler(strand, res)
                }));
                return;
            }
            owner.locked = true;
        };

        handler(Strand { io: io, value: value.clone() }, res);

        while let Some(handler) = {
            let mut owner = value.1.lock().unwrap();
            if let Some(handler) = owner.queue.pop_front() {
                Some(handler)
            } else {
                owner.locked = false;
                None
            }
        } {
            handler(io, &value);
        }
    }
}

pub struct Strand<'a, T> {
    io: &'a IoService,
    value: Value<T>,
}

impl<'a, T> Strand<'a, T> {
    pub fn new<U: IoObject>(io: &'a U, data: T) -> Strand<'a, T> {
        Strand {
            io: io.io_service(),
            value: Arc::new((UnsafeStrandCell { data: data }, Mutex::default())),
        }
    }

    pub unsafe fn get(&self) -> &mut T {
        &mut *self.value.0.get()
    }

    pub fn wrap<R, F>(&self, handler: F) -> StrandHandler<T, F, R>
        where F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
              R: Send + 'static,
    {
        StrandHandler {
            value: self.value.clone(),
            handler: handler,
            marker: PhantomData,
        }
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
        unsafe { &*self.value.0.get() }
    }
}

impl<'a, T> DerefMut for Strand<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.value.0.get() }
    }
}
