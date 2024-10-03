use std::str::FromStr;

use alpaca_to_polars::S;
use apca::data::v2::bars::{List, ListReqInit, TimeFrame};
use apca::{ApiInfo, Client, RequestError};
use chrono::{prelude::*, Months};
use error::CLIError;
use indicators::BollingerBands;
use polars::df;
use polars::prelude::*;

mod alpaca_to_polars;
mod error;
mod indicators;
mod trader;
mod traits;

async fn data_get(date: &str) -> Result<apca::data::v2::bars::Bars, CLIError> {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = DateTime::from_str(date).unwrap();
    let end = start.checked_add_months(Months::new(2)).unwrap();
    let request = ListReqInit {
        limit: Some(5),
        ..Default::default()
    }
    .init("AAPL", start, end, TimeFrame::OneDay);
    let e = client.issue::<List>(&request).await?;
    Ok(e)
}

fn data(res: apca::data::v2::bars::Bars, span: DynamicGroupOptions) -> Result<DataFrame, CLIError> {
    let bars = res.bars;
    let mut s: S = bars.into();
    s.v = s.v.with_row_index("index".into(), None).unwrap();
    let n =
        s.v.clone()
            .lazy()
            .select([col("*")])
            .group_by_dynamic(col("time"), [], span)
            .agg([col("close").mean().alias("name")])
            .collect()
            .unwrap()
            .with_row_index("index".into(), None)
            .unwrap();
    let joined =
        s.v.join(&n, ["index"], ["index"], JoinType::Left.into())
            .unwrap();
    let oo = joined
        .clone()
        .lazy()
        .select([col("*").exclude(["time_right"])])
        .collect()
        .unwrap();
    Ok(oo)
}

#[tokio::main]
async fn main() {
    let span = DynamicGroupOptions {
        index_column: PlSmallStr::from("movingAvg"),
        every: Duration::parse("1d"),
        period: Duration::parse("2d"),
        offset: Duration::parse("0d"),
        ..Default::default()
    };
    let res = data_get("2018-11-03T21:47:00Z").await.unwrap();
    let oo = data(res, span).unwrap();

    print!("{:?}", oo);
    //let df_av = s.v.lazy().w
}

//POLARS
/* let df = DataFrame::new(vec![Series::new("close".into(), b)]).unwrap();

println!("{}", df); */
