mod timer_queue;
pub use self::timer_queue::{Expiry, TimerContext, AsyncTimer, TimerQueue};

#[cfg(target_os = "linux")] mod timerfd;
#[cfg(target_os = "linux")] pub use self::timerfd::{
    TimerFdScheduler as Scheduler,
};

#[cfg(not(target_os = "linux"))] mod mediocre;
#[cfg(not(target_os = "linux"))] pub use self::mediocre::{
    MediocreScheduler as Scheduler,
};
