use crate::proto::{self};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Indi {
    pub symbol: String,
    pub indicator: HashMap<proto::IndicatorType, f64>,
}

#[derive(Clone, Debug)]
pub struct IndiValidate {
    pub validate: HashMap<String, HashMap<proto::IndicatorType, f64>>,
}

#[derive(Clone, Debug)]
pub enum ActionEval {
    Buy(f32),
    Sell(f32),
    Hold(f32),
}

#[derive(Clone, Debug)]
pub struct ActionValidate {
    pub validate: HashMap<String, ActionEval>,
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
