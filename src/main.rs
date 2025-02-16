use error::CLIError;
use std::sync::Arc;
use std::sync::Mutex;
use trader::TraderConfigs;

mod alpaca_to_polars;
mod config;
mod data;
mod dataframe;
mod error;
mod helper;
mod indicator_decision;
mod test_helper;
mod trade;
mod trader;
mod traits;
mod types;
use proto::indicator_client::IndicatorClient;

pub mod proto {
    tonic::include_proto!("calculate");
    tonic::include_proto!("plots");
}

#[tokio::main]
async fn main() -> Result<(), CLIError> {
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::fmt().compact().finish();

    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing::info!("Hello, world!");
    let client = IndicatorClient::connect("http://[::1]:50051").await?;
    tracing::info!("Hello, world!");
    let tr = TraderConfigs::new("Config.toml", Some(client), "ORCL").await?;
    let tr_config = Arc::new(Mutex::new(tr.clone()));

    //spawn trader
    let handles = tr.trader_spawn(tr_config).await;

    for i in handles {
        i.await.unwrap();
    }
    Ok(())
}
