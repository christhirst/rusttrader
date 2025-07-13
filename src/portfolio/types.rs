use std::collections::{HashMap, VecDeque};

use apca::data::v2::stream::Bar;
use serde::Deserialize;
use tracing::{error, info};

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

#[derive(Deserialize, Clone, Debug)]
pub struct Buffer {
    pub capacity: usize,
    pub data: VecDeque<Bar>,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(unused)]
pub struct TraderConf {
    pub variant: String,
    pub symbol: String,
    pub price_label: String,
    pub indicator: Vec<IndicatorType>,
    pub shares_to_buy: f64,
    //pub buffersize: usize,
    pub buff: Buffer,
}

#[derive(Clone, Debug)]
pub struct Portfolio {
    pub name: String,
    pub cash: Option<f64>,
    pub stocks: Option<HashMap<String, f64>>, // symbol and amount of shares
}
