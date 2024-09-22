use std::str::FromStr;

use apca::data::v2::bars::{self, Bar, List, ListReq, ListReqInit, TimeFrame};
use apca::{ApiInfo, Client};
use chrono::{prelude::*, Months};
use num_decimal::num_bigint::{BigInt, ToBigInt};
use num_decimal::num_rational::BigRational;
use num_decimal::Num;
use polars::functions::concat_df_horizontal;
use polars::prelude::*;
use polars::{df, frame::DataFrame};
use serde::Deserialize;
use struct_iterable::Iterable;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Iterable)]
struct sss {
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

impl From<Bar> for sss {
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

impl From<Vec<Bar>> for S {
    fn from(v: Vec<Bar>) -> Self {
        let dt: NaiveDateTime = NaiveDate::from_ymd_opt(2016, 7, 8)
            .unwrap()
            .and_hms_opt(9, 10, 11)
            .unwrap();

        let dft = df!("time" => v.iter().map(|b| dt).collect::<Vec<NaiveDateTime>>()).unwrap();

        let oo = v
            .clone()
            .iter()
            .map(|x| x.time.naive_utc())
            .collect::<Vec<NaiveDateTime>>();
        let dfts = df!("time" => oo).unwrap();

        let df = df! {
            //"time" => structToSlice("time",v.clone()),
            "open" => structToSlice("open",v.clone()),
            "close" => structToSlice("close",v.clone()),
            "high" => structToSlice("high",v.clone()),
            "low" => structToSlice("low",v.clone()),
            "volume" => structToSlice("volume",v),
        };

        let df_append = concat_df_horizontal(&[dfts, df.unwrap()], false);

        S {
            v: df_append.unwrap(),
        }
    }
    /* ... */
}

pub struct S {
    v: DataFrame,
}

impl S {
    pub fn loads(v: Vec<sss>) -> Self {
        let ii: Series =
            Series::from_iter(v.iter().map(|b| b.close.to_f64().unwrap_or_default())).into();
        println!("{}", ii);
        todo!()
    }
}

fn structToSlice(s: &str, v: Vec<Bar>) -> Series {
    let i = match s {
        ("open" | "close" | "high" | "low") => {
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
    let dt: NaiveDateTime = NaiveDate::from_ymd_opt(2016, 7, 8)
        .unwrap()
        .and_hms_opt(9, 10, 11)
        .unwrap();
    let bc = v
        .iter()
        .map(|b| {
            match s {
                //"time" => b.time,
                _ => dt,
                //"volume" => b.volume,
            }
        })
        .collect::<Vec<NaiveDateTime>>();
    let df = df!("time" => bc).unwrap();

    //Series::from_iter(i)
    i
}

#[tokio::main]
async fn main() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = DateTime::from_str("2018-11-03T21:47:00Z").unwrap();
    let end = start.checked_add_months(Months::new(2)).unwrap();
    let request = ListReqInit {
        limit: Some(5),
        ..Default::default()
    }
    .init("AAPL", start, end, TimeFrame::OneDay);

    let res = client.issue::<List>(&request).await.unwrap();
    let bars = res.bars;
    let d = bars[0].clone().time;
    //let dd = d.
    df!(""=>vec![1, 2, 3]);
    let s: S = bars.into();
    print!("{:?}", s.v);
}

//POLARS
/* let df = DataFrame::new(vec![Series::new("close".into(), b)]).unwrap();

println!("{}", df); */
