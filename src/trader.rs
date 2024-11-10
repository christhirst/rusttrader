use std::{
    collections::HashMap,
    fmt::{self},
    sync::Arc,
    thread,
    time::{self, Instant},
};

use apca::{
    api::v2::order::{self, Order, Side, Type},
    ApiInfo, Client,
};
use num_decimal::Num;
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

    async fn reconnect_client(mut self, port: &str) {
        let addr = "http://[::1]:50051";
        self.client = IndicatorClient::connect(addr).await.unwrap();
    }

    pub async fn trader_spawn(self, now: Instant) -> Vec<JoinHandle<()>> {
        let arc = Arc::new(self);
        let mut treads: Vec<JoinHandle<()>> = vec![];
        for (symbol, trader_conf) in arc.conf_map.clone() {
            let self_clone = Arc::clone(&arc);
            // No need to clone if not using Arc
            //let client = (&self.trader).clone(); // Use `clone()` if `IndicatorClient` implements Clone
            let t = tokio::spawn(async move {
                let ten_millis = time::Duration::from_millis(100);
                let now = time::Instant::now();

                thread::sleep(ten_millis);
                self_clone.trader(&trader_conf).await;
            });
            treads.push(t);
        }
        treads
    }

    async fn decision_point(
        self: Arc<Self>,
        indicator: proto::IndicatorType,
        // mut client: IndicatorClient<Channel>,
    ) -> Result<Order, CLIError> {
        let data = vec![4.0, 5.0, 6.0, 6.0, 6.0, 2.0];
        let indicate = self
            .clone()
            .grpc(indicator, String::from("ORCL"), data)
            .await;
        let desc = self.desision_maker(indicate);
        let ae = action_evaluator(desc);
        match ae.action {
            Action::Buy => stock_buy(ae).await,
            Action::Sell => stock_sell(ae).await,
            _ => todo!(),
        }
    }

    async fn grpc(
        self: Arc<Self>,
        indicator: IndicatorType,
        //ii: IndicatorClient<Channel>,
        symbol: String,
        data: Vec<f64>,
    ) -> Indi {
        let mut treads: Vec<JoinHandle<()>> = vec![];
        let mut c = self.client.clone();
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
        Indi {
            symbol, //String::from("ORCL"),
            indicator: HashMap::new(),
        }
    }

    async fn trader(self: Arc<Self>, conf: &TraderConf) {
        let self_clone = Arc::clone(&self);
        for i in conf.indicator.iter() {
            self_clone.clone().decision_point(*i).await;
        }
    }

    fn desision_maker(self: Arc<Self>, indicator: Indi) -> Vec<Action> {
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
            }
            None => action.push(Action::Hold),
        };
        action
    }
}

struct Indi {
    symbol: String,
    indicator: HashMap<proto::IndicatorType, f64>,
}

fn action_evaluator(av: Vec<Action>) -> ActionValuator {
    let buy_count = av.iter().filter(|x| **x == Action::Buy).count();
    let sell_count = av.iter().filter(|x| **x == Action::Sell).count();
    if buy_count > sell_count * 2 {
        ActionValuator {
            symbol: String::from("ORCL"),
            strength: 0.1,
            action: Action::Buy,
        }
    } else {
        ActionValuator {
            symbol: String::from("ORCL"),
            strength: 0.2,
            action: Action::Buy,
        }
    }
}

async fn stock_buy(av: ActionValuator) -> Result<Order, CLIError> {
    let amount = (av.strength * 10.0) as i64;
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let request = order::CreateReqInit {
        type_: Type::Limit,
        limit_price: Some(Num::from(100)),
        ..Default::default()
    }
    .init(av.symbol, Side::Buy, order::Amount::quantity(amount));

    let order = client.issue::<order::Create>(&request).await?;
    println!("order: {:#?}", order);
    Ok(order)
}
async fn stock_sell(av: ActionValuator) -> Result<Order, CLIError> {
    let amount = (av.strength * 10.0) as i64;
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let request = order::CreateReqInit {
        type_: Type::Limit,
        limit_price: Some(Num::from(100)),
        ..Default::default()
    }
    .init(av.symbol, Side::Buy, order::Amount::quantity(amount));

    let order = client.issue::<order::Create>(&request).await?;
    println!("order: {:#?}", order);
    Ok(order)
}

#[derive(Clone, PartialEq)]
enum Action {
    Buy,
    Sell,
    Hold,
}

#[derive(Clone)]
pub struct ActionValuator {
    symbol: String,
    strength: f64,
    action: Action,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desision_maker_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut gg = vec![HashMap::from([(proto::IndicatorType::BollingerBands, 0.1)])];

        let hm = Indi {
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

    #[test]
    fn action_evaluator_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut gg = vec![Action::Buy, Action::Buy, Action::Buy, Action::Sell];

        action_evaluator(gg);
        //findReplace(hay, r"^ki");
        //let result = 2 + 2;
        let o = AppConfig::default();
        println!("{:?}", conf);
        assert_eq!(conf, o);
        Ok(())
    }
}
