extern crate time;
extern crate asio;
use std::io;
use std::sync::Arc;
use asio::*;
use time::Duration;

static mut goal_flag: bool = false;

fn start(io: &IoService) {
    let timer = Arc::new(SystemTimer::new(io));
    timer.async_wait_for(Duration::nanoseconds(1), bind(on_nano_wait, &timer));
}

fn on_nano_wait(timer: Arc<SystemTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        timer.async_wait_for(Duration::milliseconds(2), bind(on_milli_wait, &timer));
    } else {
        panic!();
    }
}

fn on_milli_wait(timer: Arc<SystemTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        timer.async_wait_for(Duration::seconds(3), bind(on_sec_wait, &timer));
    } else {
        panic!();
    }
}

fn on_sec_wait(_: Arc<SystemTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        unsafe { goal_flag = true; }
    } else {
        panic!();
    }
}

#[test]
fn main() {
    let io = &IoService::new();
    start(io);
    io.run();
    assert!(unsafe { goal_flag });
}
