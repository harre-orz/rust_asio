use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeadlineClock(Option<Instant>);

impl DeadlineClock {
    pub fn now() -> Self {
	Self(Some(Instant::now()))
    }
    
    pub fn infinity() -> Self {
	Self(None)
    }

    pub fn into_poll(self) -> i32 {
	if let Some(deadline) = self.0 {
	    deadline.duration_since(Instant::now()).as_millis() as i32
	} else {
	    -1
	}
    }

    pub fn into_itimerspec(self) -> Option<libc::itimerspec> {
	None
	// if let Some(deadline) = self.0 {
        //     libc::itimerspec {
	// 	it_interval: libc::timespec {
        //             tv_sec: 0,
        //             tv_nsec: 0,
	// 	},
	// 	it_value: self.tv,
        //     }
	// } else {
	//     None
	// }
    }

    pub fn is_older_than(&self, now: Instant) -> bool {
	if let Some(deadline) = &self.0 {
	    *deadline < now
	} else {
	    false
	}
    }
}

impl From<Instant> for DeadlineClock {
    fn from(instant: Instant) -> Self {
	Self(Some(instant))
    }
}

// impl cmp::Eq for DeadlineClock {}

// impl cmp::PartialEq for DeadlineClock {
//     fn eq(&self, other: &Self) -> bool {
//         self.tv.tv_sec == other.tv.tv_sec && self.tv.tv_nsec == other.tv.tv_nsec
//     }
// }

// impl cmp::Ord for DeadlineClock {
//     fn cmp(&self, other: &Self) -> cmp::Ordering {
//         match self.tv.tv_sec.cmp(&other.tv.tv_sec) {
//             cmp::Ordering::Equal => self.tv.tv_nsec.cmp(&other.tv.tv_nsec),
//             cmp => cmp,
//         }
//     }
// }

// impl cmp::PartialOrd for DeadlineClock {
//     fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
//         Some(self.cmp(&other))
//     }
// }

// impl fmt::Debug for DeadlineClock {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}.{:09}", self.tv.tv_sec, self.tv.tv_nsec)
//     }
// }

// impl Add<Timeout> for Deadline {
//     type Output = Self;

//     fn add(self, rhs: Timeout) -> Self {
//         let nsec = self.tv.tv_nsec as u64 + rhs.as_nanos();
//         Self {
//             tv: libc::timespec {
//                 tv_sec: self.tv.tv_sec + (nsec / 1_000_000_000) as i64,
//                 tv_nsec: (nsec % 1_000_000_000) as i64,
//             },
//         }
//     }
// }

// impl Add<Duration> for Monotonic {
//     type Output = Self;

//     fn add(self, rhs: Duration) -> Self {
//         self + Timeout::from(rhs)
//     }
// }

// #[test]
// fn test_timeout_at_1s() {
//     let now = Monotonic::now();
//     let t = now + Duration::new(1, 0);
//     assert_eq!(t.timeout_at(now).as_millis_i32(), 1000);
//     assert_eq!(now.timeout_at(t).as_millis_i32(), 0);
// }

// #[test]
// fn test_timeout_at_overflow() {
//     let now = Monotonic::now();
//     let t = Monotonic {
//         tv: libc::timespec {
//             tv_sec: i64::MAX,
//             tv_nsec: 999_999_999,
//         },
//     };
//     assert_eq!(t.timeout_at(now).as_millis_i32(), -1);
//     assert_eq!(now.timeout_at(t).as_millis_i32(), 0);
// }
