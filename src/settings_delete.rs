use std::env;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub enum Action {
    Long,
    Short,
    Hold,
    All,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
enum Indicator {
    Bol,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum IndicatorType {
    BollingerBands = 0,
    ExponentialMovingAverage = 1,
    MaxDrawdown = 2,
    MaxDrawup = 3,
    Maximum = 4,
    MeanAbsoluteDeviation = 5,
    Minimum = 6,
    RateOfChange = 7,
    RelativeStrengthIndex = 8,
    SimpleMovingAverage = 9,
    StandardDeviation = 10,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Stockconfig {
    pub name: String,
    pub symbol: String,
    pub indicator: Vec<IndicatorType>,
    pub action: Action,
    pub buffersize: usize,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AppConfig {
    pub grpcport: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct Settings {
    pub grpc: AppConfig,
    pub Stockconfig: Vec<Stockconfig>,
}

/* impl Settings {
    pub(crate) fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            // Start off by merging in the "default" configuration file
            .add_source(File::with_name("config/default"))
            // Add in the current environment file
            // Default to 'development' env
            // Note that this file is _optional_
            .add_source(File::with_name(&format!("config/{run_mode}")).required(false))
            // Add in a local configuration file
            // This file shouldn't be checked in to git
            .add_source(File::with_name("config/local").required(false))
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
            .add_source(Environment::with_prefix("app"))
            // You may also programmatically change settings
            .set_override("database.url", "postgres://")?
            .build()?;

        // Now that we're done, let's access our configuration

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_deserialize()
    }
}
 */
