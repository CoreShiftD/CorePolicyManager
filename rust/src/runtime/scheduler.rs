use std::time::{Duration, Instant};

pub struct Scheduler {
    last_tick: Instant,
    tick_interval: Duration,
    pub ticks: u64,
}

impl Scheduler {
    pub fn new(interval: Duration) -> Self {
        Self {
            last_tick: Instant::now(),
            tick_interval: interval,
            ticks: 0,
        }
    }

    pub fn timeout_until_next_tick_ms(&self) -> i32 {
        let elapsed = self.last_tick.elapsed();
        if elapsed >= self.tick_interval {
            0
        } else {
            (self.tick_interval - elapsed).as_millis() as i32
        }
    }

    pub fn should_tick(&self) -> bool {
        self.last_tick.elapsed() >= self.tick_interval
    }

    pub fn tick(&mut self) {
        self.last_tick = Instant::now();
        self.ticks += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_timeout() {
        let interval = Duration::from_millis(100);
        let scheduler = Scheduler::new(interval);

        let timeout = scheduler.timeout_until_next_tick_ms();
        assert!(timeout > 0);
        assert!(timeout <= 100);

        std::thread::sleep(Duration::from_millis(110));
        assert_eq!(scheduler.timeout_until_next_tick_ms(), 0);
        assert!(scheduler.should_tick());
    }
}
