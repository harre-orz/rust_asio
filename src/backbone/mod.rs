use std::io;
use std::boxed::FnBox;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Condvar};
use {IoService};
use ops::*;

pub type Handler = Box<FnBox(io::Result<()>) + Send + 'static>;

mod expiry;
pub use self::expiry::*;

mod task_executor;
pub use self::task_executor::*;

mod timer_queue;
pub use self::timer_queue::*;

mod epoll_reactor;
pub use self::epoll_reactor::*;

struct BackboneCache {
    handler_vec: Vec<Handler>,
}

struct EventFd {
    intr: EpollIntrActor,
    running: bool,
}

pub struct Backbone {
    task: TaskExecutor,
    queue: TimerQueue,
    epoll: EpollReactor,
    event_fd: Mutex<EventFd>,
}

impl Backbone {
    pub fn new() -> io::Result<Backbone> {
        let event_fd = {
            let fd = try!(eventfd(0));
            EpollIntrActor::new(fd)
        };
        Ok(Backbone {
            task: TaskExecutor::new(),
            queue: TimerQueue::new(),
            epoll: try!(EpollReactor::new()),
            event_fd: Mutex::new(EventFd {
                intr: event_fd,
                running: false
            }),
        })
    }

    pub fn interrupt(io: &IoService) {
        let mut event_fd = io.0.event_fd.lock().unwrap();
        if event_fd.running {
            send(&event_fd.intr, &[1,0,0,0,0,0,0,0], 0);
        } else {
            event_fd.running = true;
            Self::dispatch(&io, Box::new(BackboneCache {
                handler_vec: Vec::new(),
            }));
        }
    }

    pub fn stop(io: &IoService) {
        TaskExecutor::stop(io);
        let mut event_fd = io.0.event_fd.lock().unwrap();
        if event_fd.running {
            // TODO: release actor of epoll_reactor and timer_queue.
            send(&event_fd.intr, &[1,0,0,0,0,0,0,0], 0);
        }
    }

    fn dispatch(io: &IoService, mut data: Box<BackboneCache>) {
        let _io = io;
        let io = io.clone();
        _io.post(move || {
            let epoll: &EpollReactor = io.use_service();
            let timer: &TimerQueue = io.use_service();
            let expiry = timer.first_timeout();

            let ready
                = epoll.poll(&expiry, &mut data.handler_vec)
                + timer.drain_expired(&mut data.handler_vec);
            for callback in data.handler_vec.drain(..) {
                io.post(move || {
                    callback(Ok(()))
                });
            }

            if ready > 0 {
                Self::dispatch(&io, data);
            } else {
                let mut event_fd = io.0.event_fd.lock().unwrap();
                event_fd.running = false;
            }
        });
    }
}

pub trait UseService<T : Sized> {
    fn use_service(&self) -> &T;
}

impl UseService<TaskExecutor> for IoService {
    fn use_service(&self) -> &TaskExecutor {
        &self.0.task
    }
}

impl UseService<TimerQueue> for IoService {
    fn use_service(&self) -> &TimerQueue {
        &self.0.queue
    }
}

impl UseService<EpollReactor> for IoService {
    fn use_service(&self) -> &EpollReactor {
        &self.0.epoll
    }
}
