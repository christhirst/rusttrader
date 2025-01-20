use error::CLIError;
use trader::TraderConfigs;

mod alpaca_to_polars;
mod config;
mod dataframe;
mod error;
mod helper;
mod indicator_decision;
mod test_helper;
mod trade;
mod trader;
mod traits;
mod types;

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

    let tr = TraderConfigs::new("Config.toml", "ORCL").await?;
    let handles = tr.trader_spawn().await;

    for i in handles {
        i.await.unwrap();
    }
    Ok(())
}
