//no warnings
#![allow(warnings)]

use apca::data::v2::stream::Data;
use error::CLIError;
use std::sync::Arc;
use std::sync::Mutex;
use trader::TraderConfigs;

mod alpaca_to_polars;
mod client;
mod config;
mod config2;
mod data;
mod dataframe;
mod error;
mod helper;
mod indicator_decision;
mod runner;
mod test_helper;
mod trade;
mod trader;
mod types;
use proto::indicator_client::IndicatorClient;
mod portfolio;

pub mod proto {
    tonic::include_proto!("calculate");
    tonic::include_proto!("plots");
}

mod settings_delete;
//use settings::Settings;
use config2::Settings;

use apca::data::v2::stream::drive;
use apca::data::v2::stream::Bar;
use apca::data::v2::stream::MarketData;
use apca::data::v2::stream::Quote;
use apca::data::v2::stream::RealtimeData;
use apca::data::v2::stream::IEX;
use apca::ApiInfo;
use apca::Client;
use apca::Error;

use chrono::DateTime;
use chrono::Utc;

use futures::FutureExt as _;
use futures::StreamExt as _;
use futures::TryStreamExt as _;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trade {
    /// The trade's symbol.
    #[serde(rename = "S")]
    pub symbol: String,
    /// The trade's ID.
    #[serde(rename = "i")]
    pub trade_id: u64,
    /// The trade's price.
    #[serde(rename = "p")]
    pub trade_price: Num,
    /// The trade's size.
    #[serde(rename = "s")]
    pub trade_size: u64,
    /// The trade's conditions.
    #[serde(rename = "c")]
    pub conditions: Vec<String>,
    /// The trade's time stamp.
    #[serde(rename = "t")]
    pub timestamp: DateTime<Utc>,
    /// The trade's exchange.
    #[serde(rename = "x")]
    pub exchange: String,
    /// The trade's tape.
    #[serde(rename = "z")]
    pub tape: String,
    /// The trade's update, may be "canceled", "corrected", or
    /// "incorrect".
    #[serde(rename = "u", default)]
    pub update: Option<String>,
}

fn te(data: Data<Bar, Quote, Trade>) -> () {}

#[tokio::main]
async fn main() -> Result<(), CLIError> {
    // Requires the following environment variables to be present:
    // - APCA_API_KEY_ID -> your API key
    // - APCA_API_SECRET_KEY -> your secret key
    //
    // Optionally, the following variable is honored:
    // - APCA_API_BASE_URL -> the API base URL to use (set to
    //   https://api.alpaca.markets for live trading)
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let (mut stream, mut subscription) = client
        .subscribe::<RealtimeData<IEX, Bar, Quote, Trade>>()
        .await
        .unwrap();

    let mut data = MarketData::default();
    // Subscribe to minute aggregate bars for SPY and XLK...
    data.set_bars(["SPY", "XLK"]);
    // ... and realtime quotes for AAPL...
    data.set_quotes(["AAPL"]);
    // ... and realtime trades for TSLA.
    data.set_trades(["TSLA"]);

    let subscribe = subscription.subscribe(&data).boxed();
    // Actually subscribe with the websocket server.
    let () = drive(subscribe, &mut stream)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let () = stream
        // Stop after receiving and printing 50 updates.
        .take(50)
        .map_err(Error::WebSocket)
        .try_for_each(|result| async { result.map(|data| te(data)).map_err(Error::Json) })
        .await
        .unwrap();

    //let settings = Settings::new();

    // Print out our settings
    //println!("{settings:?}");

    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::fmt().compact().finish();

    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing::info!("Hello, world!");

    let settings = Settings::new().unwrap();

    let client = IndicatorClient::connect("http://[::1]:50051").await?;
    tracing::info!("Hello, world!");
    let tr = TraderConfigs::new(settings, "Config.toml", Some(client), "ORCL").await?;
    let tr_config = Arc::new(Mutex::new(tr.clone()));

    //spawn trader
    let handles = tr.trader_spawn(tr_config).await;

    for i in handles {
        i.await.unwrap();
    }
    Ok(())
}
