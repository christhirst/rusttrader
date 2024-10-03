use std::fmt;

use crate::error::{Result, TaError};
use crate::traits::{Next, Reset};
use chrono::{DateTime, Duration, Utc};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[doc(alias = "EMA")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct ExponentialMovingAverage {
    duration: Duration,
    k: f64,
    window: VecDeque<(DateTime<Utc>, f64)>,
    current: f64,
    is_new: bool,
}

impl ExponentialMovingAverage {
    pub fn new(duration: Duration) -> Result<Self> {
        if duration.num_days() == 0 {
            Err(TaError::InvalidParameter)
        } else {
            Ok(Self {
                duration,
                k: 2.0 / (duration.num_days() as f64 + 1.0),
                window: VecDeque::new(),
                current: 0.0,
                is_new: true,
            })
        }
    }

    fn remove_old_data(&mut self, current_time: DateTime<Utc>) {
        while self
            .window
            .front()
            .map_or(false, |(time, _)| *time <= current_time - self.duration)
        {
            self.window.pop_front();
        }
    }
}

impl Next<f64> for ExponentialMovingAverage {
    type Output = f64;

    fn next(&mut self, (timestamp, value): (DateTime<Utc>, f64)) -> Self::Output {
        self.remove_old_data(timestamp);
        self.window.push_back((timestamp, value));

        if self.is_new {
            self.is_new = false;
            self.current = value;
        } else {
            // The weight should be constant and equal to `k`
            self.current = (self.k * value) + ((1.0 - self.k) * self.current);
        }
        self.current
    }
}

impl Reset for ExponentialMovingAverage {
    fn reset(&mut self) {
        self.window.clear();
        self.current = 0.0;
        self.is_new = true;
    }
}

impl Default for ExponentialMovingAverage {
    fn default() -> Self {
        Self::new(Duration::days(14)).unwrap()
    }
}

impl fmt::Display for ExponentialMovingAverage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "EMA({} days)", self.duration.num_days())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_new() {
        assert!(ExponentialMovingAverage::new(Duration::days(0)).is_err());
        assert!(ExponentialMovingAverage::new(Duration::days(1)).is_ok());
    }

    #[test]
    fn test_next() {
        let mut ema = ExponentialMovingAverage::new(Duration::days(3)).unwrap();
        let now = Utc::now();

        assert_eq!(ema.next((now, 2.0)), 2.0);
        assert_eq!(ema.next((now + Duration::days(1), 5.0)), 3.5);
        assert_eq!(ema.next((now + Duration::days(2), 1.0)), 2.25);
        assert_eq!(ema.next((now + Duration::days(3), 6.25)), 4.25);
    }

    #[test]
    fn test_reset() {
        let mut ema = ExponentialMovingAverage::new(Duration::days(5)).unwrap();
        let now = Utc::now();

        assert_eq!(ema.next((now, 4.0)), 4.0);
        ema.next((now + Duration::days(1), 10.0));
        ema.next((now + Duration::days(2), 15.0));
        ema.next((now + Duration::days(3), 20.0));
        assert_ne!(ema.next((now + Duration::days(4), 4.0)), 4.0);

        ema.reset();
        assert_eq!(ema.next((now, 4.0)), 4.0);
    }

    #[test]
    fn test_default() {
        let _ema = ExponentialMovingAverage::default();
    }

    #[test]
    fn test_display() {
        let ema = ExponentialMovingAverage::new(Duration::days(7)).unwrap();
        assert_eq!(format!("{}", ema), "EMA(7 days)");
    }
}
