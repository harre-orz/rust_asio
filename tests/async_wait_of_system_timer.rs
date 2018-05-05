extern crate asyncio;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use asyncio::*;

static mut GOAL_FLAG: bool = false;

fn on_nano_wait(timer: Arc<SystemTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        println!("on_nano_wait");
        timer.expires_from_now(Duration::new(0, 1000000));
        timer.async_wait(wrap(&timer, on_milli_wait));
    } else {
        panic!("{:?}", res);
    }
}

fn on_milli_wait(timer: Arc<SystemTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        println!("on_milli_wait");
        timer.expires_from_now(Duration::new(1, 0));
        timer.async_wait(wrap(&timer, on_sec_wait));
    } else {
        panic!("{:?}", res);
    }
}

fn on_sec_wait(_: Arc<SystemTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        println!("on_sec_wait");
        unsafe {
            GOAL_FLAG = true;
        }
    } else {
        panic!("{:?}", res);
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    let timer = Arc::new(SystemTimer::new(ctx));
    timer.expires_from_now(Duration::new(0, 1));
    timer.async_wait(wrap(&timer, on_nano_wait));
    ctx.run();
    assert!(unsafe { GOAL_FLAG });
}
