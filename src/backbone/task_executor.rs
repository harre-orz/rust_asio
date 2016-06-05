use std::boxed::FnBox;
use std::collections::{VecDeque, HashMap};
use std::sync::{Mutex, Condvar};
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

    pub fn post_strand_id<T: UseService<Self>>(io: &T, callback: TaskHandler, id: usize) {
        let mut task = io.use_service().mutex.lock().unwrap();
        if id > 0 {
            if let Some(ref mut queue) = task.strand_queue.get_mut(&id) {
                queue.push_back(callback);
                return;
            }
            let _ = task.strand_queue.insert(id, VecDeque::new());
        }
        task.ready_queue.push_back((id, callback));
        io.use_service().condvar.notify_one();
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
            } else if let Some(callback) = task.ready_queue.pop_front() {
                return Some(callback);
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
            assert!(task.strand_queue.remove(&id).is_some());
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

#[test]
fn test_ready_queue() {
    use IoService;
    fn queue_len<T: UseService<TaskExecutor>>(io: &T) -> usize {
        let task = io.use_service().mutex.lock().unwrap();
        task.ready_queue.len()
    }

    let io = IoService::new();
    assert!(queue_len(&io) == 0);
    io.post(|| {});
    assert!(queue_len(&io) == 1);
    io.post(|| {});
    assert!(queue_len(&io) == 2);
    io.post(|| {});
    assert!(queue_len(&io) == 3);
    assert!(TaskExecutor::run_one(&io) == 1);
    assert!(queue_len(&io) == 2);
    assert!(TaskExecutor::run_one(&io) == 1);
    assert!(queue_len(&io) == 1);
    assert!(TaskExecutor::run_one(&io) == 1);
    assert!(queue_len(&io) == 0);
    assert!(TaskExecutor::run_one(&io) == 0);
}

#[test]
fn test_strand_queue() {
    use IoService;
    const ID0:usize = 0;
    const ID1:usize = 100;
    const ID2:usize = 200;
    fn queue_len<T: UseService<TaskExecutor>>(io: &T, id: usize) -> (usize, usize, usize) {
        let task = io.use_service().mutex.lock().unwrap();
        (task.ready_queue.len(), task.strand_queue.len(), if let Some(ref queue) = task.strand_queue.get(&id) { queue.len() } else { 0 })
    }

    let io = IoService::new();
    assert!(queue_len(&io, ID0) == (0,0,0));
    TaskExecutor::post_strand_id(&io, Box::new(|| {}), ID0);
    assert!(queue_len(&io, ID0) == (1,0,0));
    TaskExecutor::post_strand_id(&io, Box::new(|| {}), ID1);
    assert!(queue_len(&io, ID1) == (2,1,0));
    TaskExecutor::post_strand_id(&io, Box::new(|| {}), ID1);
    assert!(queue_len(&io, ID1) == (2,1,1));
    io.run_one();  // consume ID0
    assert!(queue_len(&io, ID1) == (1,1,1));
    TaskExecutor::post_strand_id(&io, Box::new(|| {}), ID2);
    assert!(queue_len(&io, ID2) == (2,2,0));
    io.run_one();  // consume ID1
    assert!(queue_len(&io, ID1) == (2,2,0));
    io.run_one();  // consume ID1
    assert!(queue_len(&io, ID1) == (1,1,0));
    io.run_one();  // consume ID2
    assert!(queue_len(&io, ID1) == (0,0,0));
}
