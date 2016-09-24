extern crate time;
extern crate asyncio;
use std::io;
use std::sync::Arc;
use time::Duration;
use asyncio::*;

static mut goal_flag: bool = false;

fn start(io: &IoService) {
    let timer = Arc::new(SteadyTimer::new(io));
    timer.async_wait_for(Duration::nanoseconds(1), bind(on_nano_wait, &timer));
}

fn on_nano_wait(timer: Arc<SteadyTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        timer.async_wait_for(Duration::milliseconds(2), bind(on_milli_wait, &timer));
    } else {
        panic!();
    }
}

fn on_milli_wait(timer: Arc<SteadyTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        timer.async_wait_for(Duration::seconds(3), bind(on_sec_wait, &timer));
    } else {
        panic!();
    }
}

fn on_sec_wait(_: Arc<SteadyTimer>, res: io::Result<()>) {
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
