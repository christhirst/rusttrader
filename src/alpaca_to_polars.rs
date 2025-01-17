use apca::data::v2::bars::Bar;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use num_decimal::Num;
use polars::{
    df, frame::DataFrame, functions::concat_df_horizontal, prelude::NamedFrom, series::Series,
};
use serde::Deserialize;
use struct_iterable::Iterable;

use crate::error::CLIError;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Iterable)]
pub struct Sss {
    /// The beginning time of this bar.
    #[serde(rename = "t")]
    pub time: DateTime<Utc>,
    /// The open price.
    #[serde(rename = "o")]
    pub open: Num,
    /// The close price.
    #[serde(rename = "c")]
    pub close: Num,
    /// The highest price.
    #[serde(rename = "h")]
    pub high: Num,
    /// The lowest price.
    #[serde(rename = "l")]
    pub low: Num,
    /// The trading volume.
    #[serde(rename = "v")]
    pub volume: usize,
}

impl From<Bar> for Sss {
    fn from(a: Bar) -> Self {
        Self {
            time: a.time,
            open: a.open,
            close: a.close,
            high: a.high,
            low: a.low,
            volume: a.volume,
        }
    }
}

pub struct S {
    pub v: DataFrame,
}

impl S {
    /* pub fn loads(v: Vec<Sss>) -> Self {
        let ii: Series = Series::from_iter(v.iter().map(|b| b.close.to_f64().unwrap_or_default()));
        println!("{}", ii);
        todo!()
    } */
}

impl From<Vec<Bar>> for S {
    fn from(v: Vec<Bar>) -> Self {
        let dt: NaiveDateTime = NaiveDate::from_ymd_opt(2016, 7, 8)
            .unwrap()
            .and_hms_opt(9, 10, 11)
            .unwrap();

        //let dft = df!("time" => v.iter().map(|b| dt).collect::<Vec<NaiveDateTime>>()).unwrap();

        let oo = v
            .clone()
            .iter()
            .map(|x| x.time.naive_utc())
            .collect::<Vec<NaiveDateTime>>();

        //polars dataframe syntax
        let df = df! {
            //"time" => struct_to_slice("time",v.clone()),
            "open"=> struct_to_slice("open",v.clone()).unwrap(),
            "close" => struct_to_slice("close",v.clone()).unwrap(),
            "high" => struct_to_slice("high",v.clone()).unwrap(),
            "low" => struct_to_slice("low",v.clone()).unwrap(),
            "volume" => struct_to_slice("volume",v).unwrap(),
        };
        let dfts = df!("time" => oo).unwrap();

        let df_append = concat_df_horizontal(&[dfts, df.unwrap()], false);

        S {
            v: df_append.unwrap(),
        }
    }
    /* ... */
}

fn struct_to_slice(s: &str, v: Vec<Bar>) -> Result<Series, CLIError> {
    let i = match s {
        "open" | "close" | "high" | "low" => {
            Series::from_iter(
                v.iter()
                    .map(|b| {
                        match s {
                            //"time" => b.time,
                            "open" => b.open.to_f64().unwrap_or_default(),
                            "close" => b.close.to_f64().unwrap_or_default(),
                            "high" => b.high.to_f64().unwrap_or_default(),
                            "low" => b.low.to_f64().unwrap_or_default(),
                            _ => 1.0,
                            //"volume" => b.volume,
                        }
                    })
                    .collect::<Vec<f64>>(),
            )
        }
        "volume" => Series::from_iter(
            v.iter()
                .map(|b| match s {
                    "volume" => b.volume as i32,
                    _ => 0,
                })
                .collect::<Vec<i32>>(),
        ),
        _ => Series::new("".into(), Vec::<f64>::new()),
    };
    let dt = NaiveDate::from_ymd_opt(2016, 7, 8)
        .unwrap()
        .and_hms_opt(9, 10, 11)
        .ok_or_else(|| CLIError::ConvertingError)?;
    let bc = v.iter().map(|_| dt).collect::<Vec<NaiveDateTime>>();
    let df = df!("time" => bc).unwrap();

    //Series::from_iter(i)
    Ok(i)
}
