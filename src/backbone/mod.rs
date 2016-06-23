use std::io;
use std::boxed::FnBox;
use std::sync::Mutex;
use IoService;
use ops::*;

pub enum HandlerResult {
    Ready,
    Canceled,
}

pub type Handler = Box<FnBox(*const IoService, HandlerResult) + Send + 'static>;

pub type TaskHandler = Box<FnBox(*const IoService) + Send + 'static>;

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
    polling: bool,
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
                polling: false,
                event_fd: event_fd,
                timer_fd: timer_fd,
            }),
        })
    }

    pub fn stop(&self) {
        self.task.stop();
        self.interrupt();
    }

    fn interrupt(&self) {
        let ctrl = self.ctrl.lock().unwrap();
        if ctrl.polling {
            write(&ctrl.event_fd, &[1,0,0,0,0,0,0,0]).unwrap();
        }
    }

    fn reset_timeout(&self, expiry: Expiry) {
        let ctrl = self.ctrl.lock().unwrap();
        if ctrl.polling {
            let new_value = itimerspec {
                it_interval: timespec { tv_sec: 0, tv_nsec: 0 },
                it_value: expiry.wait_monotonic_timespec(),
            };
            timerfd_settime(&ctrl.timer_fd, TFD_TIMER_ABSTIME, &new_value).unwrap();
        }
    }

    pub fn post(&self, id: usize, callback: TaskHandler) {
        self.task.post(id, callback);
    }

    pub fn run(io: &IoService) {
        if {
            let mut ctrl = io.0.ctrl.lock().unwrap();
            if ctrl.polling {
                false
            } else {
                ctrl.event_fd.set_intr(io);
                ctrl.timer_fd.set_intr(io);
                ctrl.polling = true;
                true
            }
        } {
            Self::dispatch(io, Box::new(BackboneCache {
                handler_vec: Vec::new(),
            }));
        }

        while let Some((id, callback)) = io.0.task.do_run_one() {
            callback(io);
            io.0.task.pop(id);
        }
    }

    fn dispatch(io: &IoService, mut data: Box<BackboneCache>) {
        if io.stopped() {
            io.0.epoll.drain_all(&mut data.handler_vec);
            io.0.queue.drain_all(&mut data.handler_vec);
            for (id, callback) in data.handler_vec.drain(..) {
                io.0.task.post(id, Box::new(move |io| callback(io, HandlerResult::Canceled)));
            }

            let mut ctrl = io.0.ctrl.lock().unwrap();
            ctrl.polling = false;
            ctrl.event_fd.unset_intr(&io);
            ctrl.timer_fd.unset_intr(&io);
        } else {
            io.post(move |io| {
                let block = io.0.task.is_work();
                let mut count = io.0.epoll.poll(block, &mut data.handler_vec);
                count += io.0.queue.drain_expired(&mut data.handler_vec);
                count += data.handler_vec.len();
                for (id, callback) in data.handler_vec.drain(..) {
                    io.0.task.post(id, Box::new(move |io| callback(io, HandlerResult::Ready)));
                }

                if !block && count == 0 && io.0.task.count() == 0 {
                    io.0.task.stop();
                }

                Self::dispatch(&io, data);
            });
        }
    }
}
