extern crate asyncio;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use asyncio::*;

static mut GOAL_COUNT: usize = 0;

fn on_wait(_: Arc<SteadyTimer>, res: io::Result<()>) {
    if let Ok(_) = res {
        println!("on_wait {}", unsafe { GOAL_COUNT });
        unsafe {
            GOAL_COUNT += 1;
        }
    } else {
        panic!("{:?}", res);
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    for t in 0..10 {
        let timer = Arc::new(SteadyTimer::new(ctx));
        timer.expires_from_now(Duration::new(0, t * 1000));
        timer.async_wait(wrap(on_wait, &timer));
    }
    ctx.run();
    assert_eq!(unsafe { GOAL_COUNT }, 10);
}
