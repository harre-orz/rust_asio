extern crate time;
extern crate asio;
use std::io;
use asio::*;
use time::Duration;

struct FooTimer {
    timer: SteadyTimer,
}

impl FooTimer {
    fn start(io: &IoService) {
        let my = Strand::new(io, FooTimer {
            timer: SteadyTimer::new(io),
        });
        SteadyTimer::async_wait_for(|my| &my.timer, &Duration::nanoseconds(1), Self::on_nano_wait, &my);
    }

    fn on_nano_wait(my: Strand<FooTimer>, res: io::Result<()>) {
        if let Ok(_) = res {
            SteadyTimer::async_wait_for(|my| &my.timer, &Duration::milliseconds(2), Self::on_milli_wait, &my);
        } else {
            panic!();
        }
    }

    fn on_milli_wait(my: Strand<FooTimer>, res: io::Result<()>) {
        if let Ok(_) = res {
            SteadyTimer::async_wait_for(|my| &my.timer, &Duration::seconds(3), Self::on_sec_wait, &my);
        } else {
            panic!();
        }
    }

    fn on_sec_wait(_: Strand<FooTimer>, res: io::Result<()>) {
        if let Ok(_) = res {
        } else {
            panic!();
        }
    }
}

#[test]
fn tests_async_timer() {
    let io = IoService::new();
    FooTimer::start(&io);
    io.run();
}
