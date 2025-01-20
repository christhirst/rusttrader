//#[feature(arbitrary_self_types)]
use apca::api::v2::order::Order;

use polars::{io::SerReader, prelude::CsvReadOptions};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use std::{
    collections::HashMap,
    fmt::{self},
    sync::Arc,
    vec,
};
use tokio::task::JoinHandle;
use tonic::transport::Channel;

use crate::{
    config::AppConfig,
    dataframe::{data_select_column, data_select_column1},
    error::CLIError,
    helper::desision_maker,
    indicator_decision::action_evaluator,
    proto::{self, indicator_client::IndicatorClient, IndicatorType},
    trade::{stock_buy, stock_sell},
    types::{Action, Indi, TraderConf},
};

#[derive(Clone, Debug)]
pub struct TraderConfigs {
    conf_map: HashMap<String, TraderConf>,
    client: Option<IndicatorClient<Channel>>,
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
    pub async fn new(path: &str, sym: &str) -> Result<Self, CLIError> {
        let default_conf = AppConfig::default();
        let conf = default_conf.confload(path).unwrap();

        let mut create_trader = HashMap::new();
        let tc = TraderConf {
            symbol: String::from(sym),
            indicator: vec![
                (proto::IndicatorType::BollingerBands),
                (proto::IndicatorType::SimpleMovingAverage),
            ],
        };
        create_trader.insert(sym.to_string(), tc);
        let port = conf.grpcport;
        //setup multiple trader
        //benchmark them against each other
        match IndicatorClient::connect(port.clone()).await {
            Ok(_client) => Ok(TraderConfigs {
                conf_map: create_trader,
                client: Some(IndicatorClient::connect(port).await?),
            }),
            Err(_e) => Ok(TraderConfigs {
                conf_map: create_trader,
                client: None,
            }),
        }
    }

    #[allow(dead_code)]
    async fn reconnect_client(mut self, port: &str) {
        let mut addr = String::from("http://[::1]:");
        addr.push_str(port);
        self.client = Some(IndicatorClient::connect(addr).await.unwrap());
    }

    //TODO CancellationToken
    pub async fn trader_spawn(self) -> Vec<JoinHandle<()>> {
        //token for shutdown
        let shotdown_token = CancellationToken::new();

        let conf = Arc::new(self);
        let mut treads: Vec<JoinHandle<()>> = vec![];
        for (_symbol, trader_conf) in conf.conf_map.clone() {
            //token for shutdown cloned
            let cloned_shotdown_token = shotdown_token.clone();

            let self_clone = Arc::clone(&conf);
            // No need to clone if not using Arc
            //let client = (&self.trader).clone(); // Use `clone()` if `IndicatorClient` implements Clone
            let t = tokio::spawn(async move {
                tokio::select! {
                    // Step 3: Using cloned token to listen to cancellation requests
                    _ = cloned_shotdown_token.cancelled() => {
                        // The token was cancelled, task can shut down
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {

                self_clone.trader(&trader_conf).await;
                        // Long work has completed
                    }
                }

                /* let ten_millis = time::Duration::from_millis(100);

                thread::sleep(ten_millis);
                self_clone.trader(&trader_conf).await; */
            });
            treads.push(t);
        }
        //Cancel the original or cloned token to notify other tasks about shutting down gracefully
        shotdown_token.cancel();
        treads
    }

    /* async fn data_append(
        self: Arc<Self>,
        data: DataFrame,
        av: (String, Vec<f64>),
    ) -> Result<Vec<f64>, CLIError> {
        let df = data_append(data, av)?;
        //df.with_column(column("Close").cast::<Float64>())
        //Date,Open,High,Low,Close,Adj Close,Volume
        let close: Vec<f64> = df["Close"].f64().unwrap().to_vec_null_aware().unwrap_left();
        Ok(close)
    } */

    async fn decision_point(
        self: Arc<Self>,
        indicator: proto::IndicatorType,
    ) -> Result<Order, CLIError> {
        //let data = self.clone();
        let df = CsvReadOptions::default()
            .try_into_reader_with_file_path(Some("files/orcl.csv".into()))
            .unwrap()
            .finish()?;
        let datsa = data_select_column("ORCL", df)?;
        let close = data_select_column1(datsa, "Close")?;
        let sym = String::from("ORCL");
        let indicate = self.clone().grpc(indicator, sym.clone(), close).await;
        print!("indicate: {:?}", indicate);
        //TODO add dates
        let indicator_selected = vec![proto::IndicatorType::BollingerBands];
        let desc = desision_maker(indicate, indicator_selected);
        let ae = action_evaluator(sym, desc);
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
        let tracker = TaskTracker::new();
        let mut treads: Vec<JoinHandle<()>> = vec![];
        let mut c = self.client.clone().unwrap();
        let handle = tokio::spawn(async move {
            println!("now running on a worker thread");
            let opt = proto::Opt {
                multiplier: 1.0,
                period: 2,
            };
            let req = proto::ListNumbersRequest2 {
                id: indicator.into(),
                opt: Some(opt),
                list: data,
            };
            let request = tonic::Request::new(req);
            c.gen_liste(request).await.unwrap();
        });
        treads.push(handle);

        for i in treads {
            i.await.unwrap();
        }
        // Once we spawned everything, we close the tracker.
        tracker.close();

        // Wait for everything to finish.
        tracker.wait().await;

        Indi {
            symbol, //String::from("ORCL"),
            indicator: HashMap::new(),
        }
    }

    async fn trader(self: Arc<Self>, trader_conf: &TraderConf) {
        let self_clone = Arc::clone(&self);
        for i in trader_conf.indicator.iter() {
            //get data from csv or grpc
            //check with indicator
            //evaluate action
            //execute action
            let _ = self_clone.clone().decision_point(*i).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new() -> Result<(), Box<dyn std::error::Error>> {
        let tr = TraderConfigs::new("Config.toml", "ORCL").await?;

        let foo = Arc::new(tr);

        /* let req = proto::ListNumbersRequest2 {
            id: indicator.into(),
            list: data,
        }; */

        foo.data_indicator_get(req);

        Ok(())
    }

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
}

//PUT/CALL ratio, VIX, AAII Sentiment, Fear and Greed Index
