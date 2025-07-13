use apca::data::v2::stream::Bar;

use crate::proto::{self};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub struct Indi {
    pub symbol: String,
    pub indicator: HashMap<proto::IndicatorType, f64>,
}

//hashmap of symbol and all indicators with strength
#[derive(Clone, Debug)]
pub struct IndiValidate {
    pub validate: HashMap<String, HashMap<proto::IndicatorType, f64>>,
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub capacity: usize,
    pub data: VecDeque<Bar>,
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

#[derive(Clone, Debug)]
pub struct ActionConfig {
    pub action_validate: Option<ActionValidate>,
    pub indi_validate: Option<IndiValidate>,
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

#[derive(Clone, Debug)]
pub struct TradeConfig {
    pub symbol: String,
    pub strength: f64,
    pub action: Action,
}
