use std::{
    collections::VecDeque,
    str::FromStr,
    thread::{self, sleep},
    time,
};

use apca::data::v2::{
    quotes::Quote,
    stream::{Bar, Data, Trade},
};
use chrono::{Date, DateTime, Duration, TimeZone, Utc};
use num_decimal::Num;
use num_rational::BigRational;
use polars::{
    error::PolarsError,
    frame::DataFrame,
    io::SerReader,
    prelude::{
        col, duration, lit, ChunkCast, Column, CsvReadOptions, DataType, Float64Chunked, IntoLazy,
        NamedFrom, NonExistent, TimeUnit,
    },
    series::{IntoSeries, Series},
};

use crate::error::CLIError;
//Data<Bar, Quote, Trade>
pub fn data_csv(filename: String) -> Result<DataFrame, CLIError> {
    let df = CsvReadOptions::default()
        .map_parse_options(|parse_options| parse_options.with_try_parse_dates(true))
        .try_into_reader_with_file_path(Some(filename.into()))
        .unwrap()
        .finish()?;
    let df = df
        .lazy()
        .with_column(col("Date").cast(DataType::Datetime(TimeUnit::Milliseconds, None)))
        .collect()?;

    Ok(df)
}

fn testtt(i: &mut f64) -> f64 {
    //*i = *i + 1;
    println!("{:?}", i);
    2.0
}

// Your custom transformation function
// Returning closures from functions
pub fn make_adder() -> impl FnMut(DateTime<Utc>, f64, f64, f64, f64) -> f64 {
    //Data<Bar, Quote, Trade>
    let mut count = 0;
    move |d: DateTime<Utc>, o: f64, c: f64, h: f64, l: f64| {
        println!("o:{o}");
        println!("c:{c}");
        println!("l:{l}");
        println!("h:{h}");
        o + l + testtt(&mut c.clone())
    } // 'move' captures n by value
}

pub fn trader(d: DateTime<Utc>, o: f64, c: f64, h: f64, l: f64) -> f64 {
    let bar_new = Bar {
        symbol: "ORCL".to_string(),
        open_price: Num::from_str(&o.to_string()).unwrap(),
        high_price: Num::from_str(&h.to_string()).unwrap(),
        low_price: Num::from_str(&l.to_string()).unwrap(),
        close_price: Num::from_str(&c.to_string()).unwrap(),
        volume: Num::from(100),
        timestamp: d,
    };
    let mut buffer: VecDeque<Bar> = VecDeque::with_capacity(5);

    // If the buffer already has 5 items, remove the oldest (front) item.
    if buffer.len() == 5 {
        buffer.pop_front();
    }
    // Add the new value to the back of the buffer.
    buffer.push_back(bar_new);
    println!("Buffer length: {}", buffer.len());
    2.0
}

pub fn data_stream(filename: String) -> Data<Bar, Quote, Trade> {
    // Define a closure that takes two f64 values and returns an f64
    let mut add = make_adder();

    let mut df = data_csv(String::from("files/orcl.csv")).unwrap();
    //timestamp: DateTime<Utc>
    // Parse the "date" column as Utf8 (string), then convert to DateTime<Utc>

    let date = df.column("Date").unwrap().datetime().unwrap();

    //.naive_utc()
    println!("{:?}", df);
    //panic!("test)");

    let open = df.column("Open").unwrap().f64().unwrap();
    let close = df.column("Close").unwrap().f64().unwrap();
    let high = df.column("High").unwrap().f64().unwrap();
    let low = df.column("Low").unwrap().f64().unwrap();
    //let Volume = df.column("Volume").unwrap().f64().unwrap();

    // Perform the operation
    let values: Vec<Option<f64>> = close
        .into_iter()
        .zip(open.into_iter())
        .zip(high.into_iter())
        .zip(low.into_iter())
        .zip(date.into_iter())
        .map(|((((opt_c, opt_l), opt_h), opt_o), opt_d)| {
            match (opt_d, opt_l, opt_h, opt_o, opt_c) {
                (Some(d), Some(o), Some(h), Some(l), Some(c)) => {
                    Some(trader(Utc.timestamp_opt(d, 0).unwrap(), o, c, h, l))
                }
                _ => None,
            }
        })
        .collect();
    /* let new_series = Series::new("MyNewColumn".into(), values);
    // Add the new column to the DataFrame
    df.with_column(new_series).unwrap(); */

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
    use std::vec;

    use polars::prelude::{col, StrptimeOptions};

    use super::*;

    #[test]
    fn trader2_test() -> Result<(), Box<dyn std::error::Error>> {
        let ee = vec![
            (1.0, 2.0, 3.0, 4.0, 5.0),
            (2.0, 3.0, 4.0, 5.0, 6.0),
            (3.0, 4.0, 5.0, 6.0, 7.0),
            (1.0, 2.0, 3.0, 4.0, 5.0),
            (2.0, 3.0, 4.0, 5.0, 6.0),
            (3.0, 4.0, 5.0, 6.0, 7.0),
        ];
        for i in ee {
            let res = trader(Utc::now(), i.0, i.1, i.2, i.3);
            println!("Result: {}", res);
        }
        assert!("false" == "true");
        Ok(())
    }

    #[tokio::test]
    async fn data_get_test() -> Result<(), Box<dyn std::error::Error>> {
        let df = data_csv(String::from("files/orcl.csv"));
        println!("{:?}", df);
        assert!(!df.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn data_stream_test() -> Result<(), Box<dyn std::error::Error>> {
        let df = data_stream(String::from("files/orcl.csv"));
        println!("{:?}", df);

        Ok(())
    }
}
