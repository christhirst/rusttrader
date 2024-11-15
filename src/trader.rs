#[feature(arbitrary_self_types)]
use apca::{
    api::v2::order::{self, Order, Side, Type},
    ApiInfo, Client,
};
use num_decimal::Num;
use polars::{
    io::SerReader,
    prelude::{CsvReadOptions, DataFrame, NamedFrom},
    series::Series,
};
use tracing_subscriber::registry::Data;

use std::{
    collections::HashMap,
    fmt::{self},
    sync::Arc,
    thread,
    time::{self, Instant},
    vec,
};
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
    pub async fn new(path: &str) -> Result<Self, CLIError> {
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
        Ok(TraderConfigs {
            conf_map: create_trader,
            client: IndicatorClient::connect(port).await?,
        })
    }

    #[allow(dead_code)]
    async fn reconnect_client(mut self, port: &str) {
        let mut addr = String::from("http://[::1]:");
        addr.push_str(port);
        self.client = IndicatorClient::connect(addr).await.unwrap();
    }

    pub async fn trader_spawn(self) -> Vec<JoinHandle<()>> {
        let arc = Arc::new(self);
        let mut treads: Vec<JoinHandle<()>> = vec![];
        for (symbol, trader_conf) in arc.conf_map.clone() {
            let self_clone = Arc::clone(&arc);
            // No need to clone if not using Arc
            //let client = (&self.trader).clone(); // Use `clone()` if `IndicatorClient` implements Clone
            let t = tokio::spawn(async move {
                let ten_millis = time::Duration::from_millis(100);

                thread::sleep(ten_millis);
                self_clone.trader(&trader_conf).await;
            });
            treads.push(t);
        }
        treads
    }

    async fn data_get(self: Arc<Self>, symbol: &str) -> Result<Vec<f64>, CLIError> {
        let df = CsvReadOptions::default()
            .try_into_reader_with_file_path(Some("files/orcl.csv".into()))
            .unwrap()
            .finish()?;
        //df.with_column(column("Close").cast::<Float64>())
        //Date,Open,High,Low,Close,Adj Close,Volume
        let close: Vec<f64> = df["Close"].f64().unwrap().to_vec_null_aware().unwrap_left();
        Ok(close)
    }

    async fn data_append(
        self: Arc<Self>,
        data: DataFrame,
        av: (String, Vec<f64>),
    ) -> Result<Vec<f64>, CLIError> {
        let df = data_append(data, av).await?;
        //df.with_column(column("Close").cast::<Float64>())
        //Date,Open,High,Low,Close,Adj Close,Volume
        let close: Vec<f64> = df["Close"].f64().unwrap().to_vec_null_aware().unwrap_left();
        Ok(close)
    }

    async fn decision_point(
        self: Arc<Self>,
        indicator: proto::IndicatorType,
    ) -> Result<Order, CLIError> {
        let data = self.clone();

        let datsa = data.data_get("ORCL").await?;
        let indicate = self
            .clone()
            .grpc(indicator, String::from("ORCL"), datsa)
            .await;
        let desc = self.desision_maker(indicate);
        let ae = action_evaluator(desc);
        match ae.action {
            Action::Buy => stock_buy(ae).await,
            Action::Sell => stock_sell(ae).await,
            _ => todo!(),
        }
    }

    async fn data_indicator_get(self: Arc<Self>, req: proto::ListNumbersRequest2) -> Vec<f64> {
        let mut c = self.client.clone();
        let request = tonic::Request::new(req);
        let ii = c.gen_liste(request).await.unwrap().into_inner().result;
        //let ii = ii.;

        ii
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

fn data_csv(filename: String) -> Result<DataFrame, CLIError> {
    //"files/orcl.csv".into()
    let df = CsvReadOptions::default()
        .try_into_reader_with_file_path(Some(filename.into()))
        .unwrap()
        .finish()?;
    Ok(df)
    // let s0 = Series::new(av., av.values().cloned().collect::<Vec<f64>>());
}

async fn data_append(mut df: DataFrame, av: (String, Vec<f64>)) -> Result<DataFrame, CLIError> {
    let i = df.with_column(Series::new(av.0.into(), av.1)).cloned()?;
    Ok(i)
    // let s0 = Series::new(av., av.values().cloned().collect::<Vec<f64>>());
}

async fn df_to_vec(df: DataFrame, column: &str) -> Result<Vec<f64>, CLIError> {
    let close = df[column]
        .f64()
        .unwrap()
        .to_vec_null_aware()
        .left()
        .ok_or(CLIError::ConvertingError);
    close
    // let s0 = Series::new(av., av.values().cloned().collect::<Vec<f64>>());
}

#[derive(Clone, PartialEq, Debug)]
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

    #[tokio::test]
    async fn desision_maker_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut gg = HashMap::from([(proto::IndicatorType::BollingerBands, 0.1)]);

        let hm = Indi {
            symbol: String::from("ORCL"),
            indicator: gg,
        };
        let tr = TraderConfigs::new("Config.toml").await?;
        let foo = Arc::new(tr);
        let handles = foo.desision_maker(hm);

        //findReplace(hay, r"^ki");
        //let result = 2 + 2;
        //let o = AppConfig::default();
        println!("{:?}", handles);
        //assert_eq!(conf, o);
        Ok(())
    }

    #[test]
    fn action_evaluator_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut gg = vec![Action::Buy, Action::Buy, Action::Buy, Action::Sell];

        action_evaluator(gg);
        //findReplace(hay, r"^ki");
        //let result = 2 + 2;
        //let o = AppConfig::default();
        //println!("{:?}", conf);
        //assert_eq!(conf, o);
        Ok(())
    }

    /*  #[tokio::test]
       async fn data_get_test() -> Result<DataFrame, Box<dyn std::error::Error>> {
           let tr = TraderConfigs::new("Config.toml").await?;
           let foo = Arc::new(tr);
           let handles = data_csv(String::from("files/orcl.csv"));
           println!("{:?}", handles);

           //Ok(handles)
           todo!()
       }
    */
    #[tokio::test]
    async fn data_grpc_get_test() -> Result<(), Box<dyn std::error::Error>> {
        /*  let tr = TraderConfigs::new("Config.toml").await?;
        let foo = Arc::new(tr);
        let req = proto::ListNumbersRequest2 {
            id: indicator.into(),
            list: data,
        };

        foo.data_indicator_get(req); */

        Ok(())
    }

    #[tokio::test]
    async fn data_get_append_test() -> Result<(), Box<dyn std::error::Error>> {
        //let data = data_csv(String::from("files/orcl.csv")).unwrap();
        let df = data_csv(String::from("files/orcl.csv")).unwrap();
        let tr = TraderConfigs::new("Config.toml").await?;
        let foo = Arc::new(tr);

        let data = df_to_vec(df.clone(), "Close").await?;
        let req = proto::ListNumbersRequest2 {
            id: IndicatorType::BollingerBands.into(),
            list: data,
        };
        let oo = foo.clone().data_indicator_get(req).await;
        println!("{:?}", oo);
        let ii = (String::from("ORCL"), oo);
        let oo = data_append(df, ii).await;
        println!("{:?}", oo.unwrap().head(Some(3)));
        Ok(())
    }
}
