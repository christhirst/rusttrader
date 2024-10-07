use chrono::{DateTime, Duration, Utc};
use std::collections::VecDeque;
use std::fmt;

use crate::error::Result;
use crate::indicators::StandardDeviation as Sd;
use crate::traits::{Next, Reset};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[doc(alias = "BB")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct BollingerBands {
    duration: Duration,
    multiplier: f64,
    sd: Sd,
    window: VecDeque<(DateTime<Utc>, f64)>, // Store tuples of (timestamp, value)
}

#[derive(Debug, Clone, PartialEq)]
pub struct BollingerBandsOutput {
    //moving average
    pub average: f64,
    //positiv multiplier
    pub upper: f64,
    //negativ multiplier
    pub lower: f64,
}

impl BollingerBands {
    pub fn new(duration: Duration, multiplier: f64) -> Result<Self> {
        if duration.num_seconds() <= 0 {
            return Err(crate::error::TaError::InvalidParameter);
        }
        Ok(Self {
            duration,
            multiplier,
            sd: Sd::new(duration)?, // We will manage the period dynamically
            window: VecDeque::new(),
        })
    }

    pub fn multiplier(&self) -> f64 {
        self.multiplier
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

impl Next<f64> for BollingerBands {
    type Output = f64;

    fn next(&mut self, (timestamp, value): (DateTime<Utc>, f64)) -> Self::Output {
        // Remove data points that are older than our duration
        self.remove_old_data(timestamp);

        // Add the new data point
        self.window.push_back((timestamp, value));

        // Calculate the mean and standard deviation based on the current window
        let values: Vec<f64> = self.window.iter().map(|&(_, val)| val).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let sd: f64 = self.sd.next((timestamp, value));
        mean + sd * self.multiplier
    }
}

impl Reset for BollingerBands {
    fn reset(&mut self) {
        self.sd.reset();
    }
}

impl Default for BollingerBands {
    fn default() -> Self {
        Self::new(Duration::days(14), 2_f64).unwrap()
    }
}

impl fmt::Display for BollingerBands {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BB({}, {})", self.duration, self.multiplier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::*;
    use chrono::{Duration, Utc};

    test_indicator!(BollingerBands);

    #[test]
    fn test_new() {
        assert!(BollingerBands::new(Duration::days(0), 2_f64).is_err());
        assert!(BollingerBands::new(Duration::days(1), 2_f64).is_ok());
        assert!(BollingerBands::new(Duration::days(2), 2_f64).is_ok());
    }

    #[test]
    fn test_next() {
        //Setup
        let mut bb = BollingerBands::new(Duration::days(3), 2.0).unwrap();
        let now = Utc::now();

        let a = bb.next((now, 2.0));
        let b = bb.next((now + Duration::days(1), 5.0));
        let c = bb.next((now + Duration::days(2), 1.0));
        let d = bb.next((now + Duration::days(3), 6.25));

        assert_eq!(round(a), 2.0);
        assert_eq!(round(b), 6.5);
        assert_eq!(round(c), 6.066);
        assert_eq!(round(d), 8.562);
    }

    #[test]
    fn test_reset() {
        let mut bb = BollingerBands::new(Duration::days(5), 2.0_f64).unwrap();
        let now = Utc::now();

        let out = bb.next((now, 3.0));

        assert_eq!(out, 3.0);

        bb.next((now + Duration::days(1), 2.5));
        bb.next((now + Duration::days(2), 3.5));
        bb.next((now + Duration::days(3), 4.0));

        let out = bb.next((now + Duration::days(4), 2.0));

        assert_eq!(round(out), 4.414);

        bb.reset();
        let out = bb.next((now, 3.0));
        assert_eq!(out, 3.0);
    }

    #[test]
    fn test_default() {
        BollingerBands::default();
    }

    #[test]
    fn test_display() {
        let duration = Duration::days(10);
        let bb = BollingerBands::new(duration, 3.0_f64).unwrap();
        assert_eq!(format!("{}", bb), format!("BB({}, 3)", duration));
    }
}
