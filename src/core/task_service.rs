use std::boxed::FnBox;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Condvar};

pub type TaskHandler = Box<FnBox() + Send + 'static>;

struct TaskObject {
    stopped: bool,
    blocked: bool,
    queue: VecDeque<TaskHandler>,
}

#[derive(Clone)]
pub struct TaskService {
    task: Arc<(Mutex<TaskObject>, Condvar)>
}

impl TaskService {
    pub fn new() -> TaskService {
        TaskService {
            task: Arc::new((Mutex::new(TaskObject {
                stopped: false,
                blocked: false,
                queue: VecDeque::new(),
            }), Condvar::new()))
        }
    }

    pub fn stopped(&self) -> bool {
        let task = self.task.0.lock().unwrap();
        task.stopped
    }

    pub fn stop(&self) {
        let condvar = &self.task.1;
        let mut task = self.task.0.lock().unwrap();
        if !task.stopped {
            task.stopped = true;
            condvar.notify_all();
        }
    }

    pub fn reset(&self) {
        let mut task = self.task.0.lock().unwrap();
        task.blocked = false;
        task.stopped = false;
    }

    pub fn post(&self, callback: TaskHandler) {
        let condvar = &self.task.1;
        let mut task = self.task.0.lock().unwrap();
        task.queue.push_back(callback);
        condvar.notify_one();
    }

    pub fn run(&self) -> usize {
        let mut n = 0;
        while let Some((callback, is_continue)) = self.do_run_one() {
            callback();
            n += 1;
            if !is_continue {
                break;
            }
        }
        n
    }

    pub fn run_one(&self) -> usize {
        if let Some((callback, _)) = self.do_run_one() {
            callback();
            1
        } else {
            0
        }
    }

    fn do_run_one(&self) -> Option<(TaskHandler, bool)> {
        let condvar = &self.task.1;
        let mut task = self.task.0.lock().unwrap();
        loop {
            if task.stopped {
                return None;
            } else if let Some(callback) = task.queue.pop_front() {
                return Some((callback, task.blocked || !task.queue.is_empty()));
            } else if !task.blocked {
                return None
            }
            task = condvar.wait(task).unwrap();
        }
    }

    fn block(&self) {
        let mut task = self.task.0.lock().unwrap();
        task.blocked = true;
    }

    fn clear(&self) {
        while let Some(task) = {
            let mut task = self.task.0.lock().unwrap();
            task.queue.pop_front()
        } {
            task()
        }
    }
}

pub struct TaskBlock<'a> {
    sv: &'a TaskService,
}

impl<'a> TaskBlock<'a> {
    pub fn new(sv: &'a TaskService) -> TaskBlock<'a> {
        sv.block();
        TaskBlock {
            sv: sv,
        }
    }
}

impl<'a> Drop for TaskBlock<'a> {
    fn drop(&mut self) {
        self.sv.stop();
        self.sv.clear();
    }
}

#[test]
fn test_run_one() {
    static mut flag: bool = false;
    let task = TaskService::new();
    task.post(Box::new(|| unsafe { flag = true; }));
    assert!(unsafe { flag == false });
    task.run_one();
    assert!(unsafe { flag == true });
}

#[test]
fn test_run_all() {
    static mut count: i32 = 0;
    let task = TaskService::new();
    for _ in 0..10 {
        task.post(Box::new(|| unsafe { count+= 1; }));
    }
    assert!(unsafe { count == 0 });
    task.run();
    assert!(unsafe { count == 10});
}

#[test]
fn test_stop() {
    static mut count: i32 = 0;
    let task = TaskService::new();
    for _ in 0..3 {
        let child = task.clone();
        task.post(Box::new(move || { child.stop(); unsafe { count += 1; }}));
    }
    assert!(unsafe { count == 0 });
    task.run();
    assert!(unsafe { count == 1 });
    task.run();
    assert!(unsafe { count == 1 });
}

#[test]
fn test_reset() {
    static mut count: i32 = 0;
    let task = TaskService::new();
    for _ in 0..3 {
        let child = task.clone();
        task.post(Box::new(move || { child.stop(); unsafe { count += 1; }}));
    }
    assert!(unsafe { count == 0 });
    task.run();
    assert!(unsafe { count == 1 });
    task.reset();
    task.run();
    assert!(unsafe { count == 2 });
}

#[test]
fn test_block() {
    static mut count: i32 = 0;
    let task = TaskService::new();
    for _ in 0..3 {
        let child = task.clone();
        task.post(Box::new(move || { child.stop(); unsafe { count += 1; }}));
    } {
        let block = TaskBlock::new(&task);
        assert!(unsafe { count == 0 });
    }
    assert!(unsafe { count == 3 });
}

#[test]
fn test_multi_thread() {
    use std::thread;
    let count: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let task = TaskService::new();
    {
        let block = TaskBlock::new(&task);
        let mut thrds = Vec::new();
        for _ in 0..5 {
            let count = count.clone();
            let task = task.clone();
            thrds.push(thread::spawn(move || {
                task.run();
                let count = count.lock().unwrap();
                assert!(*count == 1000);
            }));
        }

        for _ in 0..1000 {
            let count = count.clone();
            let child = task.clone();
            task.post(Box::new(move || {
                let mut count = count.lock().unwrap();
                assert!(*count < 1000);
                *count += 1;
                if *count == 1000 {
                    child.stop();
                }
            }));
        }

        for thrd in thrds {
            thrd.join();
        }
    }
}
