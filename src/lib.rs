
///
/// The atomic monitor concurrency utility, created by Phoenix Kahlo.
///

pub extern crate atomic;
pub extern crate time;

extern crate monitor;

pub use atomic::Ordering;

use atomic::Atomic;
use monitor::Monitor;
use time::{precise_time_ns, Duration};

pub struct AtomMonitor<T: Copy> {
    data: Atomic<T>,
    requesting: Atomic<u32>,
    monitor: Monitor<()>
}
impl<T: Copy> AtomMonitor<T> {
    pub fn new(value: T) -> Self {
        AtomMonitor {
            data: Atomic::new(value),
            requesting: Atomic::new(0),
            monitor: Monitor::new(())
        }
    }

    pub fn mutate<O>(&self, mut mutator: impl FnMut(&Atomic<T>) -> O) -> O {
        let out = mutator(&self.data);
        let requesting = self.requesting.load(Ordering::Acquire);
        if requesting > 0 {
            self.monitor.with_lock(|guard| guard.notify_all());
        } else {
        }
        out
    }

    pub fn get(&self) -> T {
        self.data.load(Ordering::Acquire)
    }

    pub fn set(&self, value: T) {
        self.mutate(|atomic| atomic.store(value, Ordering::Release));
    }

    pub fn wait_until(&self, mut condition: impl FnMut(T) -> bool) -> T {
        let mut value = self.get();
        if !condition(value) {
            self.requesting.fetch_add(1, Ordering::SeqCst);
            value = self.get();
            if !condition(value) {
                self.monitor.with_lock(|mut guard| {
                    while {
                        value = self.get();
                        !condition(value)
                    } {
                        guard.wait();
                    }
                });
            }
            self.requesting.fetch_sub(1, Ordering::SeqCst);
        }
        value
    }

    pub fn wait_until_timeout(&self, mut condition: impl FnMut(T) -> bool, timeout: Duration) -> Option<T> {
        let end_time = precise_time_ns() as i128 + timeout.num_nanoseconds().unwrap() as i128;

        let mut value = self.get();
        if !condition(value) {
            self.requesting.fetch_add(1, Ordering::SeqCst);
            let satisfied =
                if {
                    value = self.get();
                    condition(value)
                } {
                    true
                } else {
                    self.monitor.with_lock(|mut guard| loop {
                        if {
                            value = self.get();
                            condition(value)
                        } {
                            break true;
                        } else if precise_time_ns() as i128 > end_time {
                            break false;
                        } else {
                            if let Ok(remaining) = Duration::nanoseconds(end_time as i64 - precise_time_ns() as i64).to_std() {
                                guard.wait_timeout(remaining);
                            }
                        }
                    })
                };
            self.requesting.fetch_sub(1, Ordering::SeqCst);
            if satisfied {
                Some(value)
            } else {
                None
            }
        } else {
            Some(value)
        }
    }

    pub fn notify_all(&self) {
        self.monitor.with_lock(|guard| guard.notify_all());
    }
}