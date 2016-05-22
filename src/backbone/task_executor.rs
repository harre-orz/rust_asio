use std::boxed::FnBox;
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, Mutex, Condvar};
use {IoService, Strand};
use super::{UseService};

pub type TaskHandler = Box<FnBox() + Send + 'static>;

struct TaskQueue {
    stopped: bool,
    blocked: bool,
    ready_queue: VecDeque<(usize, TaskHandler)>,
    strand_queue: HashMap<usize, VecDeque<TaskHandler>>,
}

pub struct TaskExecutor {
    mutex: Mutex<TaskQueue>,
    condvar: Condvar,
}

impl TaskExecutor {
    pub fn new() -> TaskExecutor {
        TaskExecutor {
            mutex: Mutex::new(TaskQueue {
                stopped: false,
                blocked: false,
                ready_queue: VecDeque::new(),
                strand_queue: HashMap::new(),
            }),
            condvar: Condvar::new()
        }
    }

    pub fn stopped<T: UseService<Self>>(io: &T) -> bool {
        let task = io.use_service().mutex.lock().unwrap();
        task.stopped
    }

    pub fn stop<T: UseService<Self>>(io: &T) {
        let mut task = io.use_service().mutex.lock().unwrap();
        if !task.stopped {
            task.stopped = true;
            io.use_service().condvar.notify_all();
        }
    }

    pub fn reset<T: UseService<Self>>(io: &T) {
        let mut task = io.use_service().mutex.lock().unwrap();
        task.blocked = false;
        task.stopped = false;
    }

    pub fn post<T: UseService<Self>>(io: &T, callback: TaskHandler) {
        let mut task = io.use_service().mutex.lock().unwrap();
        task.ready_queue.push_back((0, callback));
        io.use_service().condvar.notify_one();
    }

    pub fn post_strand<T: UseService<Self>, U>(io: &T, callback: TaskHandler, obj: &Strand<U>) {
        let mut task = io.use_service().mutex.lock().unwrap();
        let id = obj.id();
        if let Some(ref mut queue) = task.strand_queue.get_mut(&id) {
            queue.push_back(callback);
            return;
        }

        let _ = task.strand_queue.insert(id, VecDeque::new());
        task.ready_queue.push_back((id, callback));
    }

    pub fn run<T: UseService<Self>>(io: &T) -> usize {
        let mut n = 0;
        while let Some((id, callback)) = io.use_service().do_run_one() {
            callback();
            io.use_service().pop_strand(id);
            n += 1;
        }
        n
    }

    pub fn run_one<T: UseService<Self>>(io: &T) -> usize {
        if let Some((id, callback)) = io.use_service().do_run_one() {
            callback();
            io.use_service().pop_strand(id);
            1
        } else {
            0
        }
    }

    fn do_run_one(&self) -> Option<(usize, TaskHandler)> {
        let mut task = self.mutex.lock().unwrap();
        loop {
            if task.stopped {
                return None;
            } else if let Some((id, callback)) = task.ready_queue.pop_front() {
                return Some((id, callback));
            } else if !task.blocked {
                return None
            }
            task = self.condvar.wait(task).unwrap();
        }
    }

    fn pop_strand(&self, id: usize) {
        if id == 0 {
            return;
        }

        let mut task = self.mutex.lock().unwrap();
        if let Some(callback) = {
            if let Some(ref mut queue) = task.strand_queue.get_mut(&id) {
                queue.pop_front()
            } else {
                return;
            }
        } {
            task.ready_queue.push_back((id, callback));
        } else {
            let _ = task.ready_queue.remove(id);
        }
    }

    pub fn block<T: UseService<Self>>(io: &T) {
        let mut task = io.use_service().mutex.lock().unwrap();
        task.blocked = true;
    }

    pub fn clear<T: UseService<Self>>(io: &T) {
        while let Some((id, callback)) = {
            let mut task = io.use_service().mutex.lock().unwrap();
            task.ready_queue.pop_front()
        } {
            callback();
            io.use_service().pop_strand(id);
        }
    }
}
