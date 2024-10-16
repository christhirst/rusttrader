use std::str::FromStr;
use std::vec;

use alpaca_to_polars::S;
use apca::data::v2::bars::{List, ListReqInit, TimeFrame};
use apca::{ApiInfo, Client};
use chrono::{prelude::*, Months};
use error::CLIError;
use polars::prelude::*;

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
}

#[tokio::main]
async fn main() -> Result<(), CLIError> {
    //CONFIG from file
    let file = "Config.toml";
    let conf = config::confload(file)?;

    let df = CsvReadOptions::default()
        .try_into_reader_with_file_path(Some("files/orcl.csv".into()))
        .unwrap()
        .finish()
        .unwrap();
    println!("{}", df);

    let addr = "http://[::1]:50051";
    let mut client = IndicatorClient::connect(addr).await.unwrap();
    let req = proto::ListNumbersRequest2 {
        id: 9,
        list: vec![4.0, 5.0, 6.0, 6.0, 6.0, 2.0],
    };
    let request = tonic::Request::new(req);
    let resp = client.gen_liste(request).await.unwrap();

    println!("{:?}", resp.get_ref().result);

    //let now = Utc::now();

    //bb.next((now, 2.0)), 2.0);

    //let res = data_get("2018-11-03T21:47:00Z").await.unwrap();

    //let df_av = s.v.lazy().w
    Ok(())
}

//POLARS
/* let df = DataFrame::new(vec![Series::new("close".into(), b)]).unwrap();

println!("{}", df); */
