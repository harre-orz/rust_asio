use std::io;
use std::boxed::FnBox;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::marker::PhantomData;
use std::collections::VecDeque;
use super::{IoObject, IoService, Strand, StrandHandler};
use unsafe_cell::{UnsafeStrandCell};
use async_result::{Handler, NullAsyncResult};

type TaskHandler<T> = Box<FnBox(*const IoService, StrandImpl<T>) + Send + 'static>;

struct StrandQueue<T> {
    locked: bool,
    queue: VecDeque<TaskHandler<T>>,
}

impl<T, F, R> Handler<R> for StrandHandler<T, F, R>
    where T: Send + 'static,
          F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
          R: Send + 'static
{
    type Output = ();

    type AsyncResult = NullAsyncResult;

    fn async_result(&self) -> Self::AsyncResult {
        NullAsyncResult
    }

    fn callback(self, io: &IoService, res: io::Result<R>) {
        let StrandHandler { owner, handler, _marker } = self;
        Strand { io: io, owner: owner }.dispatch(move |io| handler(io, res));
    }
}

pub struct StrandImpl<T>(Arc<(UnsafeStrandCell<T>, Mutex<StrandQueue<T>>)>);

impl<T> Clone for StrandImpl<T> {
    fn clone(&self) -> StrandImpl<T> {
        StrandImpl(self.0.clone())
    }
}

impl<'a, T: 'static> Strand<'a, T> {
    pub fn new<U: IoObject>(io: &'a U, data: T) -> Strand<'a, T> {
        Strand {
            io: io.io_service(),
            owner: StrandImpl(Arc::new((UnsafeStrandCell::new(data), Mutex::new(
                StrandQueue {
                    locked: false,
                    queue: VecDeque::new(),
                }
            ))))
        }
    }

    pub fn as_mut(&self) -> &mut T {
        unsafe { &mut *(self.owner.0).0.get() }
    }

    pub fn dispatch<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let _ = {
            let mut owner = (self.owner.0).1.lock().unwrap();
            if owner.locked {
                owner.queue.push_back(Box::new(move |io: *const IoService, owner: StrandImpl<T>| {
                    func(Strand { io: unsafe { &*io }, owner: owner })
                }));
                return;
            }
            owner.locked = true;
        };

        func(Strand { io: self.io, owner: self.owner.clone() });

        while let Some(func) = {
            let mut owner = (self.owner.0).1.lock().unwrap();
            if let Some(func) = owner.queue.pop_front() {
                Some(func)
            } else {
                owner.locked = false;
                None
            }
        } {
            func(self.io, self.owner.clone());
        }
    }

    pub fn post<F>(&self, func: F)
        where F: FnOnce(Strand<T>) + Send + 'static
    {
        let owner = self.owner.clone();
        self.io.post(move |io| Strand { io: io, owner: owner }.dispatch(func));
    }

    pub fn wrap<F, R>(&self, handler: F) -> StrandHandler<T, F, R>
        where F: FnOnce(Strand<T>, io::Result<R>) + Send + 'static,
              R: Send + 'static,
    {
        StrandHandler {
            owner: self.owner.clone(),
            handler: handler,
            _marker: PhantomData,
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
        unsafe { &*(self.owner.0).0.get() }
    }
}

impl<'a, T> DerefMut for Strand<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.owner.0).0.get() }
    }
}
