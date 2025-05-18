use std::collections::VecDeque;

use apca::data::v2::{
    quotes::Quote,
    stream::{Bar, Data, Trade},
};
use chrono::{DateTime, Duration, Utc};
use polars::{
    error::PolarsError,
    frame::DataFrame,
    io::SerReader,
    prelude::{Column, CsvReadOptions, Float64Chunked, IntoLazy, NamedFrom},
    series::{IntoSeries, Series},
};

use crate::error::CLIError;
//Data<Bar, Quote, Trade>
pub fn data_csv(filename: String) -> Result<DataFrame, CLIError> {
    let df = CsvReadOptions::default()
        .try_into_reader_with_file_path(Some(filename.into()))
        .unwrap()
        .finish()?;
    Ok(df)
}

fn testtt(i: &mut f64) -> f64 {
    //*i = *i + 1;
    println!("{:?}", i);
    2.0
}

// Your custom transformation function
// Returning closures from functions
pub fn make_adder() -> impl FnMut(f64, f64, f64) -> f64 {
    let mut count = 0;
    move |x: f64, y: f64, z: f64| {
        println!("{z}");
        x + y + testtt(&mut y.clone())
    } // 'move' captures n by value
}

pub fn data_stream(filename: String) -> Data<Bar, Quote, Trade> {
    // Define a closure that takes two f64 values and returns an f64
    let mut add = make_adder();

    let mut df = data_csv(String::from("files/orcl.csv")).unwrap();
    let date = df.column("High").unwrap().f64().unwrap();
    let close = df.column("Close").unwrap().f64().unwrap();
    let open = df.column("Open").unwrap().f64().unwrap();

    // Perform the operation
    let values = close
        .into_iter()
        .zip(open.into_iter())
        .zip(date.into_iter())
        .map(|((opt_x, opt_y), opt_z)| match (opt_x, opt_y, opt_z) {
            (Some(x), Some(y), Some(z)) => Some(add(x, y, z)),
            _ => None,
        })
        .collect::<Float64Chunked>()
        .into_series();
    let new_series = Series::new("MyNewColumn".into(), values);
    // Add the new column to the DataFrame
    df.with_column(new_series).unwrap();

    println!("{:?}", df);

    let d: Data<Bar, Quote, Trade> = Data::from(todo!());

    todo!()
}

pub trait Next<T> {
    type Output;
    fn next(&mut self, input: (DateTime<Utc>, T)) -> Self::Output;
}

#[derive(Debug, Clone, PartialEq)]
pub struct data {
    duration: Duration,
    //moving average
    pub currnet: f64,
    //positiv multiplier
    pub previous: f64,
    //negativ multiplier
    //pub lower: f64,
    window: VecDeque<(DateTime<Utc>, f64)>,
}

/* impl data {
    pub fn new(duration: Duration, data: String, multiplier: f64) -> Result<Self> {
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
} */

/* impl Next<f64> for data {
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
} */

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn data_get_test() -> Result<(), Box<dyn std::error::Error>> {
        let df = data_csv(String::from("files/orcl.csv"));
        println!("{:?}", df);
        assert!(df.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn data_stream_test() -> Result<(), Box<dyn std::error::Error>> {
        let df = data_stream(String::from("files/orcl.csv"));
        println!("{:?}", df);

        Ok(())
    }
}
