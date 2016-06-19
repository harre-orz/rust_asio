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

    pub fn stopped(&self) -> bool {
        let task = self.mutex.lock().unwrap();
        task.stopped
    }

    pub fn stopped_and_blocked(&self) -> (bool, bool) {
        let task = self.mutex.lock().unwrap();
        (task.stopped, task.blocked)
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
        task.blocked = false;
        task.stopped = false;
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

    pub fn run(&self) -> usize {
        let mut n = 0;
        while let Some((id, callback)) = self.do_run_one() {
            callback();
            self.pop(id);
            n += 1;
        }
        n
    }

    pub fn run_one(&self) -> usize {
        if let Some((id, callback)) = self.do_run_one() {
            callback();
            self.pop(id);
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
            } else if let Some(callback) = task.ready_queue.pop_front() {
                return Some(callback);
            } else if !task.blocked {
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

    pub fn block(&self) {
        let mut task = self.mutex.lock().unwrap();
        task.blocked = true;
    }

    pub fn clear(&self) {
        while let Some((id, callback)) = {
            let mut task = self.mutex.lock().unwrap();
            task.ready_queue.pop_front()
        } {
            callback();
            self.pop(id);
        }
    }
}

#[test]
fn test_ready_queue() {
    fn queue_len(task: &TaskExecutor) -> usize {
        let task = task.mutex.lock().unwrap();
        task.ready_queue.len()
    }

    let task = TaskExecutor::new();
    assert!(queue_len(&task) == 0);
    task.post(0, Box::new(|| {}));
    assert!(queue_len(&task) == 1);
    task.post(0, Box::new(|| {}));
    assert!(queue_len(&task) == 2);
    task.post(0, Box::new(|| {}));
    assert!(queue_len(&task) == 3);
    assert!(task.run_one() == 1);
    assert!(queue_len(&task) == 2);
    assert!(task.run_one() == 1);
    assert!(queue_len(&task) == 1);
    assert!(task.run_one() == 1);
    assert!(queue_len(&task) == 0);
    assert!(task.run_one() == 0);
}

#[test]
fn test_strand_queue() {
    const ID0:usize = 0;
    const ID1:usize = 100;
    const ID2:usize = 200;
    fn queue_len(task: &TaskExecutor, id: usize) -> (usize, usize, usize) {
        let task = task.mutex.lock().unwrap();
        (task.ready_queue.len(), task.strand_queue.len(), if let Some(ref queue) = task.strand_queue.get(&id) { queue.len() } else { 0 })
    }

    let task = TaskExecutor::new();
    assert!(queue_len(&task, ID0) == (0,0,0));
    task.post(ID0,  Box::new(|| {}));
    assert!(queue_len(&task, ID0) == (1,0,0));
    task.post(ID1, Box::new(|| {}));
    assert!(queue_len(&task, ID1) == (2,1,0));
    task.post(ID1, Box::new(|| {}));
    assert!(queue_len(&task, ID1) == (2,1,1));
    task.run_one();  // consume ID0
    assert!(queue_len(&task, ID1) == (1,1,1));
    task.post(ID2, Box::new(|| {}));
    assert!(queue_len(&task, ID2) == (2,2,0));
    task.run_one();  // consume ID1
    assert!(queue_len(&task, ID1) == (2,2,0));
    task.run_one();  // consume ID1
    assert!(queue_len(&task, ID1) == (1,1,0));
    task.run_one();  // consume ID2
    assert!(queue_len(&task, ID1) == (0,0,0));
}
