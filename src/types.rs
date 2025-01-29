use crate::proto::{self};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Indi {
    pub symbol: String,
    pub indicator: HashMap<proto::IndicatorType, f64>,
}

#[derive(Clone)]
pub struct TraderConf {
    pub symbol: String,
    pub price_label: String,
    pub indicator: Vec<proto::IndicatorType>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Action {
    Buy,
    Sell,
    Hold,
}

#[derive(Clone, Debug)]
pub struct ActionValuator {
    pub symbol: String,
    pub strength: f64,
    pub action: Action,
}
