use std::io;
use std::boxed::FnBox;
use std::sync::Mutex;
use IoService;
use ops::*;

pub enum HandlerResult {
    Ready,
    Canceled,
}

pub type Handler = Box<FnBox(HandlerResult) + Send + 'static>;

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
    pub task: TaskExecutor,
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

    pub fn stop(&self) {
        self.task.stop();
        let ctrl = self.ctrl.lock().unwrap();
        if ctrl.running {
            let _ = send(&ctrl.event_fd, &[1,0,0,0,0,0,0,0], 0);
            let mut vec = Vec::new();
            self.epoll.drain_all(&mut vec);
            self.queue.drain_all(&mut vec);
            for (id, callback) in vec {
                self.task.post(id, Box::new(move || {
                    callback(HandlerResult::Canceled);
                }));
            }
        }
    }

    fn interrupt(io: &IoService) {
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
             Self::dispatch(io, Box::new(BackboneCache {
                handler_vec: Vec::new(),
             }));
         }
    }

    fn timeout(io: &IoService, expiry: Expiry) {
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
            Self::dispatch(io, Box::new(BackboneCache {
                handler_vec: Vec::new(),
            }));
        }
    }

    fn dispatch(io_: &IoService, mut data: Box<BackboneCache>) {
        let io = io_.clone();
        io_.post(move || {
            let ready
                = io.0.epoll.poll(io.0.queue.first_timeout(), &mut data.handler_vec)
                + io.0.queue.drain_expired(&mut data.handler_vec);
            for (id, callback) in data.handler_vec.drain(..) {
                io.0.task.post(id, Box::new(move || {
                    callback(HandlerResult::Ready);
                }));
            }
            let (stopped, blocked) = io.0.task.stopped_and_blocked();
            if stopped || (ready == 0 && !blocked) {
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
