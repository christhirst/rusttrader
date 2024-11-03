use std::{
    collections::HashMap,
    fmt::{self},
    thread,
    time::{Duration, Instant},
};

use apca::data;
use tokio::task::JoinHandle;
use tonic::transport::Channel;

use crate::{
    config::AppConfig,
    error::CLIError,
    proto::{self, indicator_client::IndicatorClient, IndicatorType},
};

#[derive(Clone)]
pub struct TraderConf {
    symbol: String,
    indicator: Vec<proto::IndicatorType>,
}

#[derive(Clone, Debug)]
pub struct TraderConfigs {
    conf_map: HashMap<String, TraderConf>,
    client: IndicatorClient<Channel>,
}

impl fmt::Debug for TraderConf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "TraderConf {{ symbol: {}, indicator: {:?} }}",
            self.symbol, self.indicator
        )
    }
}

impl TraderConfigs {
    pub async fn new(path: &str) -> Self {
        let default_conf = AppConfig::default();
        let conf = default_conf.confload(path).unwrap();

        let mut create_trader = HashMap::new();
        let tc = TraderConf {
            symbol: String::from("ORCL"),
            indicator: vec![
                (proto::IndicatorType::BollingerBands),
                (proto::IndicatorType::SimpleMovingAverage),
            ],
        };
        create_trader.insert(String::from("ORCL"), tc);
        let port = conf.grpcport;
        //setup multiple trader
        //benchmark them against each other
        TraderConfigs {
            conf_map: create_trader,
            client: IndicatorClient::connect(port).await.unwrap(),
        }
    }

    /* async fn reconnect_client(mut self, port: &str) {
        let addr = "http://[::1]:50051";
        self.client = IndicatorClient::connect(addr).await.unwrap();
    } */

    pub async fn trader_spawn(self, d: Duration, now: Instant) -> Vec<JoinHandle<()>> {
        let mut treads: Vec<JoinHandle<()>> = vec![];
        for (symbol, trader_conf) in self.conf_map {
            let ee = self.client.clone();
            let f = trader;
            let t = tokio::spawn(async move {
                //println!("Hello, world! {:?}", now.elapsed());
                //thread::sleep(d);
                f(trader_conf.clone(), ee).await;
                // some work here
            });
            treads.push(t);
        }

        treads
    }
}

struct indi {
    symbol: String,
    indicator: HashMap<proto::IndicatorType, f64>,
}

async fn grpc(
    indicator: IndicatorType,
    ii: IndicatorClient<Channel>,
    symbol: String,
    data: Vec<f64>,
) -> indi {
    let mut treads: Vec<JoinHandle<()>> = vec![];
    let mut c = ii.clone();
    let handle = tokio::spawn(async move {
        println!("now running on a worker thread");
        let req = proto::ListNumbersRequest2 {
            id: indicator.into(),
            list: data,
        };
        let request = tonic::Request::new(req);
        c.gen_liste(request).await.unwrap();
    });
    treads.push(handle);

    for i in treads {
        i.await.unwrap();
    }
    indi {
        symbol, //String::from("ORCL"),
        indicator: HashMap::new(),
    }
}

fn action_evaluator(av: Vec<Action>) -> ActionValuator {
    todo!()
}

async fn decision_point(
    indicator: proto::IndicatorType,
    mut client: IndicatorClient<Channel>,
) -> Result<(), CLIError> {
    let data = vec![4.0, 5.0, 6.0, 6.0, 6.0, 2.0];
    let indicate = grpc(indicator, client.clone(), String::from("ORCL"), data).await;
    let desc = desision_maker(indicate);
    let ae = action_evaluator(desc);
    match ae.action {
        Action::Buy => stock_buy(ae),
        Action::Sell => stock_sell(ae),
        _ => Ok(()),
    }
}

async fn trader(conf: TraderConf, mut client: IndicatorClient<Channel>) {
    for i in conf.indicator.iter() {
        decision_point(*i, client.clone()).await;
    }
}

fn stock_buy(av: ActionValuator) -> Result<(), CLIError> {
    todo!()
}
fn stock_sell(av: ActionValuator) -> Result<(), CLIError> {
    todo!()
}

#[derive(Clone)]
enum Action {
    Buy,
    Sell,
    Hold,
}

#[derive(Clone)]
pub struct ActionRequest {
    symbol: String,
    action: Action,
}

#[derive(Clone)]
pub struct IndicatorValuator {
    symbol: String,
    strength: f64,
    action: proto::IndicatorType,
}

#[derive(Clone)]
pub struct ActionValuator {
    symbol: String,
    strength: f64,
    action: Action,
}

fn desision_maker(indicator: indi) -> Vec<Action> {
    let mut action = vec![];
    match indicator
        .indicator
        .get(&proto::IndicatorType::BollingerBands)
    {
        Some(x) => {
            if *x > 0.1 {
                action.push(Action::Buy)
            } else {
                action.push(Action::Sell)
            }
            action.push(Action::Hold)
        }
        None => action.push(Action::Hold),
    };
    action
}

fn send_data() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desision_maker_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut gg = vec![HashMap::from([(proto::IndicatorType::BollingerBands, 0.1)])];

        let hm = indi {
            symbol: String::from("ORCL"),
            indicator: gg,
        };

        desision_maker(hm);
        //findReplace(hay, r"^ki");
        //let result = 2 + 2;
        let o = AppConfig::default();
        println!("{:?}", conf);
        assert_eq!(conf, o);
        Ok(())
    }
}
