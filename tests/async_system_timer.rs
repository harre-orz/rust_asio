extern crate time;
extern crate asio;
use std::io;
use std::sync::Arc;
use asio::*;
use time::Duration;

static mut goal_flag: bool = false;

struct FooTimer {
    timer: SystemTimer,
}

impl FooTimer {
    fn start(io: &IoService) {
        let obj = Arc::new(FooTimer {
            timer: SystemTimer::new(io),
        });
        obj.timer.async_wait_for(Duration::nanoseconds(1), bind(Self::on_nano_wait, &obj));
    }

    fn on_nano_wait(obj: Arc<Self>, res: io::Result<()>, _: &IoService) {
        if let Ok(_) = res {
            obj.timer.async_wait_for(Duration::milliseconds(2), bind(Self::on_milli_wait, &obj));
        } else {
            panic!();
        }
    }

    fn on_milli_wait(obj: Arc<Self>, res: io::Result<()>, _: &IoService) {
        if let Ok(_) = res {
            obj.timer.async_wait_for(Duration::seconds(3), bind(Self::on_sec_wait, &obj));
        } else {
            panic!();
        }
    }

    fn on_sec_wait(_: Arc<Self>, res: io::Result<()>, _: &IoService) {
        if let Ok(_) = res {
            unsafe { goal_flag = true; }
        } else {
            panic!();
        }
    }
}

#[test]
fn main() {
    let io = IoService::new();
    FooTimer::start(&io);
    io.run();
    assert!(unsafe { goal_flag });
}
