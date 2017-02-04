extern crate asyncio;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use asyncio::*;

static mut GOAL_FLAG: bool = false;

fn on_nano_wait(timer: Arc<Mutex<SystemTimer>>, res: io::Result<()>) {
    if let Ok(_) = res {
        println!("on_nano_wait");
        timer.lock().unwrap()
            .expires_from_now(Duration::new(0, 1000000))
            .async_wait(wrap(on_milli_wait, &timer));
    } else {
        panic!("{:?}", res);
    }
}

fn on_milli_wait(timer: Arc<Mutex<SystemTimer>>, res: io::Result<()>) {
    if let Ok(_) = res {
        println!("on_milli_wait");
        timer.lock().unwrap()
            .expires_from_now(Duration::new(1, 0))
            .async_wait(wrap(on_sec_wait, &timer));
    } else {
        panic!("{:?}", res);
    }
}

fn on_sec_wait(_: Arc<Mutex<SystemTimer>>, res: io::Result<()>) {
    if let Ok(_) = res {
        println!("on_sec_wait");
        unsafe { GOAL_FLAG = true; }
    } else {
        panic!("{:?}", res);
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    let timer = Arc::new(Mutex::new(SystemTimer::new(ctx)));
    timer.lock().unwrap()
        .expires_from_now(Duration::new(0, 1))
        .async_wait(wrap(on_nano_wait, &timer));
    ctx.run();
    assert!(unsafe { GOAL_FLAG });
}
