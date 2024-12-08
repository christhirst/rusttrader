use std::str::FromStr;

use alpaca_to_polars::S;
use apca::data::v2::bars::{List, ListReqInit, TimeFrame};
use apca::{ApiInfo, Client};
use chrono::{prelude::*, Months};
use error::CLIError;
use polars::prelude::*;
use std::time::{Duration, Instant};
use trader::TraderConfigs;

mod alpaca_to_polars;
mod config;
mod error;
mod test_helper;
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

use proto::indicator_client::IndicatorClient;

pub mod proto {
    tonic::include_proto!("calculate");
    tonic::include_proto!("plots");
}

#[tokio::main]
async fn main() -> Result<(), CLIError> {
    let tr = TraderConfigs::new("Config.toml").await?;
    let handles = tr.trader_spawn().await;

    for i in handles {
        i.await.unwrap();
    }
    Ok(())
}
