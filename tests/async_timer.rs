extern crate time;
extern crate asio;
use std::io;
use asio::*;
use time::Duration;

static mut goal_flag: bool = false;

struct FooTimer {
    timer: SteadyTimer,
}

impl FooTimer {
    fn start(io: &IoService) {
        let obj = Strand::new(io, FooTimer {
            timer: SteadyTimer::new(io),
        });
        unsafe { obj.timer.async_wait_for(&Duration::nanoseconds(1), Self::on_nano_wait, &obj); }
    }

    fn on_nano_wait(obj: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            unsafe { obj.timer.async_wait_for(&Duration::milliseconds(2), Self::on_milli_wait, &obj); }
        } else {
            panic!();
        }
    }

    fn on_milli_wait(obj: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            unsafe { obj.timer.async_wait_for(&Duration::seconds(3), Self::on_sec_wait, &obj); }
        } else {
            panic!();
        }
    }

    fn on_sec_wait(_: Strand<Self>, res: io::Result<()>) {
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
