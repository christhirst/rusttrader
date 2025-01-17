#[derive(Debug, PartialEq)]
pub struct Bar {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

impl Bar {
    pub fn new() -> Self {
        Self {
            open: 0.0,
            close: 0.0,
            low: 0.0,
            high: 0.0,
            volume: 0.0,
        }
    }

    pub fn high<T: Into<f64>>(mut self, val: T) -> Self {
        self.high = val.into();
        self
    }

    pub fn low<T: Into<f64>>(mut self, val: T) -> Self {
        self.low = val.into();
        self
    }

    pub fn close<T: Into<f64>>(mut self, val: T) -> Self {
        self.close = val.into();
        self
    }

    pub fn volume(mut self, val: f64) -> Self {
        self.volume = val;
        self
    }
}

/* pub fn round(num: f64) -> f64 {
    (num * 1000.0).round() / 1000.00
} */

macro_rules! test_indicator {
    ($i:tt) => {
        #[test]
        fn test_indicator() {
            use chrono::TimeZone; // Import TimeZone trait to use the Utc.ymd method

            let bar = Bar::new();

            // Create a fixed timestamp for testing
            let timestamp = Utc.ymd(2023, 1, 1).and_hms(0, 0, 0);

            // ensure Default trait is implemented
            let mut indicator = $i::default();

            // ensure Next<f64> is implemented
            // Provide a tuple with the timestamp and the value
            let first_output = indicator.next((timestamp, 12.3));

            // ensure next accepts &DataItem as well
            // You will need to modify the implementation of Next for &DataItem
            // to accept a tuple with a timestamp as well
            // For example:
            // indicator.next((timestamp, &bar));

            // ensure Reset is implemented and works correctly
            indicator.reset();
            assert_eq!(indicator.next((timestamp, 12.3)), first_output);

            // ensure Display is implemented
            format!("{}", indicator);
        }
    };
}
