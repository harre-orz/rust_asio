use std::io;
use std::boxed::FnBox;
use std::sync::Mutex;
use IoService;
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
    handler_vec: Vec<(usize, Handler)>,
}

struct BackboneCtrl {
    running: bool,
    event_fd: EpollIntrActor,
    timer_fd: EpollIntrActor,
}

pub struct Backbone {
    task: TaskExecutor,
    queue: TimerQueue,
    epoll: EpollReactor,
    ctrl: Mutex<BackboneCtrl>,
}

impl Backbone {
    pub fn new() -> io::Result<Backbone> {
        let event_fd = {
            let fd = try!(eventfd(0));
            EpollIntrActor::new(fd)
        };
        let timer_fd = {
            let fd = try!(timerfd_create(CLOCK_MONOTONIC));
            EpollIntrActor::new(fd)
        };
        Ok(Backbone {
            task: TaskExecutor::new(),
            queue: TimerQueue::new(),
            epoll: try!(EpollReactor::new()),
            ctrl: Mutex::new(BackboneCtrl {
                running: false,
                event_fd: event_fd,
                timer_fd: timer_fd,
            }),
        })
    }

    pub fn interrupt(io: &IoService) {
        if {
            let mut ctrl = io.0.ctrl.lock().unwrap();
            if ctrl.running {
                let _ = send(&ctrl.event_fd, &[1,0,0,0,0,0,0,0], 0);
                false
            } else {
                ctrl.event_fd.set_intr(io);
                ctrl.timer_fd.set_intr(io);
                ctrl.running = true;
                true
            }
        } {
            Self::dispatch(&io, Box::new(BackboneCache {
                handler_vec: Vec::new(),
            }));
        }
    }

    pub fn timeout(io: &IoService, expiry: Expiry) {
        if {
            let mut ctrl = io.0.ctrl.lock().unwrap();
            if ctrl.running {
                let new_value = itimerspec {
                    it_interval: timespec { tv_sec: 0, tv_nsec: 0 },
                    it_value: expiry.wait_monotonic_timespec(),
                };
                let _ = timerfd_settime(&ctrl.timer_fd, TFD_TIMER_ABSTIME, &new_value);
                false
            } else {
                ctrl.event_fd.set_intr(io);
                ctrl.timer_fd.set_intr(io);
                ctrl.running = true;
                true
            }
        } {
            Self::dispatch(&io, Box::new(BackboneCache {
                handler_vec: Vec::new(),
            }));
        }
    }

    pub fn stop(io: &IoService) {
        TaskExecutor::stop(io);
        let ctrl = io.0.ctrl.lock().unwrap();
        if ctrl.running {
            let _ = send(&ctrl.event_fd, &[1,0,0,0,0,0,0,0], 0);
            let mut vec = Vec::new();
            let epoll: &EpollReactor = io.use_service();
            let timer: &TimerQueue = io.use_service();
            epoll.drain_all(&mut vec);
            timer.drain_all(&mut vec);
            for (id, callback) in vec {
                TaskExecutor::post_strand_id(io, Box::new(move || {
                    callback(Err(operation_canceled()));
                }), id);
            }
        }
    }

    fn dispatch(io: &IoService, mut data: Box<BackboneCache>) {
        let _io = io;
        let io = io.clone();
        _io.post(move || {
            let epoll: &EpollReactor = io.use_service();
            let timer: &TimerQueue = io.use_service();
            let ready
                = epoll.poll(timer.first_timeout(), &mut data.handler_vec)
                + timer.drain_expired(&mut data.handler_vec);
            for (id, callback) in data.handler_vec.drain(..) {
                TaskExecutor::post_strand_id(&io, Box::new(move || {
                    callback(Ok(()))
                }), id);
            }
            if ready == 0 || TaskExecutor::stopped(&io) {
                let mut ctrl = io.0.ctrl.lock().unwrap();
                ctrl.running = false;
                ctrl.event_fd.unset_intr(&io);
                ctrl.timer_fd.unset_intr(&io);
            } else {
                Self::dispatch(&io, data);
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
