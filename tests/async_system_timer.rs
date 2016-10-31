extern crate asyncio;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use asyncio::*;

static mut GOAL_FLAG: bool = false;

fn start(io: &IoService) {
    let timer = Arc::new(SystemTimer::new(io));
    timer.async_wait_for(Duration::new(0, 1), wrap(on_nano_wait, &timer));
}

fn on_nano_wait(timer: Arc<SystemTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        timer.async_wait_for(Duration::new(0, 1000), wrap(on_milli_wait, &timer));
    } else {
        panic!();
    }
}

fn on_milli_wait(timer: Arc<SystemTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        timer.async_wait_for(Duration::new(0, 1000000), wrap(on_sec_wait, &timer));
    } else {
        panic!();
    }
}

fn on_sec_wait(_: Arc<SystemTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        unsafe { GOAL_FLAG = true; }
    } else {
        panic!();
    }
}

#[test]
fn main() {
    let io = &IoService::new();
    start(io);
    io.run();
    assert!(unsafe { GOAL_FLAG });
}
