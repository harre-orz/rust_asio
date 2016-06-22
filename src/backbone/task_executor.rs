use std::boxed::FnBox;
use std::collections::{VecDeque, HashMap};
use std::sync::{Mutex, Condvar};

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

    pub fn count(&self) -> usize {
        let task = self.mutex.lock().unwrap();
        task.ready_queue.len() + task.strand_queue.len()
    }

    pub fn stopped(&self) -> bool {
        let task = self.mutex.lock().unwrap();
        task.stopped
    }

    pub fn stop(&self) {
        let mut task = self.mutex.lock().unwrap();
        if !task.stopped {
            task.stopped = true;
            self.condvar.notify_all();
        }
    }

    pub fn reset(&self) {
        let mut task = self.mutex.lock().unwrap();
        task.stopped = false;
    }

    pub fn is_work(&self) -> bool {
        let task = self.mutex.lock().unwrap();
        task.blocked
    }

    pub fn set_work(&self, on: bool) {
        let mut task = self.mutex.lock().unwrap();
        task.blocked = on;
    }

    pub fn post(&self, id: usize, callback: TaskHandler) {
        let mut task = self.mutex.lock().unwrap();
        if id > 0 {
            if let Some(ref mut queue) = task.strand_queue.get_mut(&id) {
                queue.push_back(callback);
                return;
            }
            let _ = task.strand_queue.insert(id, VecDeque::new());
        }
        task.ready_queue.push_back((id, callback));
        self.condvar.notify_one();
    }

    pub fn run(&self) {
        while let Some((id, callback)) = self.do_run_one() {
            callback();
            self.pop(id);
        }
    }

    fn do_run_one(&self) -> Option<(usize, TaskHandler)> {
        let mut task = self.mutex.lock().unwrap();
        loop {
            if let Some(callback) = task.ready_queue.pop_front() {
                return Some(callback);
            } else if task.stopped || !task.blocked {
                return None
            }
            task = self.condvar.wait(task).unwrap();
        }
    }

    fn pop(&self, id: usize) {
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
            self.condvar.notify_one();
        } else {
            task.strand_queue.remove(&id);
        }
    }
}
