//#[feature(arbitrary_self_types)]

use chrono::Duration;
use mockall::automock;
use polars::{io::SerReader, prelude::CsvReadOptions};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{error, info};

use std::{
    collections::HashMap,
    fmt::{self},
    sync::{Arc, Mutex},
    vec,
};
use tokio::{task::JoinHandle, time::sleep};
use tonic::transport::Channel;

use crate::{
    config::AppConfig,
    dataframe::data_select_column1,
    error::CLIError,
    helper::desision_maker,
    indicator_decision::action_evaluator,
    proto::{self, indicator_client::IndicatorClient, ListNumbersRequest2},
    types::{
        ActionConfig, ActionEval, ActionValidate, ActionValuator, Indi, IndiValidate, TraderConf,
    },
};

#[automock]
trait Calc {
    async fn grpc(
        &self,
        req: ListNumbersRequest2,
        //ii: IndicatorClient<Channel>,
        symbol: String,
    ) -> Indi;

    async fn decision_point(
        self: Arc<Self>,
        conf: &TraderConf,
        req: ListNumbersRequest2,
        col: &str,
    ) -> Result<ActionValuator, CLIError>;
}

#[derive(Clone, Debug)]
pub struct TraderConfigs {
    conf_map: HashMap<String, TraderConf>,
    client: Option<IndicatorClient<Channel>>,
    stock_indicators: Option<ActionConfig>,
}

impl Calc for TraderConfigs {
    async fn grpc(
        &self,
        req: ListNumbersRequest2,
        //ii: IndicatorClient<Channel>,
        symbol: String,
    ) -> Indi {
        let tracker = TaskTracker::new();
        let mut treads: Vec<JoinHandle<()>> = vec![];

        let mut c = self.client.clone().unwrap();

        let handle = tokio::spawn(async move {
            println!("now running on a worker thread");

            let test = c.gen_liste(req).await.unwrap();
            //println!("test: {:#?}", test);
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

    async fn decision_point(
        self: Arc<Self>,
        conf: &TraderConf,
        req: ListNumbersRequest2,
        col: &str,
    ) -> Result<ActionValuator, CLIError> {
        //get indicator data
        let indicator_from_stock_data = self.clone().grpc(req, conf.symbol.clone()).await;
        //let indicator_from_stock_data = self.clone().grpc(req, conf.symbol.clone()).await;
        println!("decision_point");
        let validators = self.clone().stock_indicators.clone();
        println!("self: {:#?}", self);
        let validators = validators.clone().unwrap();
        let validators = validators.indi_validate.unwrap().validate;

        //get stock actions
        /* let stock_actions = match self.stock_indicators.clone() {
            Some(x) => x,
            None => {
                let mut self_clone = Arc::clone(&self);
                Arc::get_mut(&mut self_clone)
                    .unwrap()
                    .pull_stock_data()
                    .await?
            }
        }; */

        //getting indicator data
        //let validators = stock_actions.indi_validate.unwrap().validate;

        let desc = desision_maker(
            indicator_from_stock_data,
            validators.get(&conf.symbol).unwrap().clone(),
        );

        Ok(action_evaluator(
            conf.symbol.clone(),
            self.stock_indicators
                .clone()
                .unwrap()
                .action_validate
                .unwrap(),
            desc,
        ))
    }
}

/* match ae.action {
    Action::Buy => self.stock_buy(ae).await,
    Action::Sell => self.stock_sell(ae).await,
    Action::Hold => {
        info!("Hold");
        Ok(())
    }
    _ => {
        error!("Invalid Action");
        Err(CLIError::Converting)
    }
} */

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
    pub async fn new(
        path: &str,
        client: Option<IndicatorClient<Channel>>,
        sym: &str,
    ) -> Result<Self, CLIError> {
        let default_conf = AppConfig::default();
        let conf = default_conf.confload(path).unwrap();

        let mut create_trader = HashMap::new();
        let tc = TraderConf {
            symbol: String::from(sym),
            price_label: String::from("Close"),
            indicator: vec![
                (proto::IndicatorType::BollingerBands),
                (proto::IndicatorType::SimpleMovingAverage),
            ],
        };
        create_trader.insert(sym.to_string(), tc);
        let _port = conf.grpcport;
        tracing::info!("Port: {}", _port);
        //setup multiple trader
        //benchmark them against each other

        let ac = ActionConfig {
            action_validate: Some(ActionValidate {
                validate: HashMap::from([
                    (String::from("ORCL"), ActionEval::Buy(0.0)),
                    (String::from("ORCL"), ActionEval::Sell(0.0)),
                    (String::from("ORCL"), ActionEval::Hold(0.0)),
                ]),
            }),
            indi_validate: Some(IndiValidate {
                validate: HashMap::from([(
                    String::from("ORCL"),
                    HashMap::from([(proto::IndicatorType::BollingerBands, 0.0)]),
                )]),
            }),
        };

        if let Some(client) = client {
            Ok(TraderConfigs {
                conf_map: create_trader,
                client: Some(client),
                stock_indicators: Some(ac),
            })
        } else {
            Err(CLIError::Converting)
        }
    }

    #[allow(dead_code)]
    async fn reconnect_client(mut self, port: &str) {
        let mut addr = String::from("http://[::1]:");
        addr.push_str(port);
        self.client = Some(IndicatorClient::connect(addr).await.unwrap());
    }

    #[allow(dead_code)]
    async fn pull_stock_data(&mut self) -> Result<ActionConfig, CLIError> {
        //TODO pull data from database

        let ii = ActionConfig {
            action_validate: Some(ActionValidate {
                validate: HashMap::from([
                    (String::from("ORCL"), ActionEval::Buy(0.1)),
                    (String::from("ORCL"), ActionEval::Sell(0.2)),
                    (String::from("ORCL"), ActionEval::Hold(0.3)),
                ]),
            }),
            indi_validate: Some(IndiValidate {
                validate: HashMap::from([(
                    String::from("ORCL"),
                    HashMap::from([(proto::IndicatorType::BollingerBands, 0.1)]),
                )]),
            }),
        };
        self.stock_indicators = Some(ii.clone());

        Ok(ii)
    }

    //TODO CancellationToken
    pub async fn trader_spawn(self, trader_conf: Arc<Mutex<TraderConfigs>>) -> Vec<JoinHandle<()>> {
        // Token for shutdown
        let shutdown_token = CancellationToken::new();
        let conf = Arc::new(self);
        let mut threads: Vec<JoinHandle<()>> = vec![];

        let conf_map = {
            let self_lock = trader_conf.lock().unwrap();
            self_lock.conf_map.clone()
        };

        //spawn trader for every symbol
        for (_symbol, trader_conf) in conf_map {
            // Token for shutdown cloned
            let cloned_shutdown_token = shutdown_token.clone();
            let self_clone = Arc::clone(&conf);

            let t = tokio::spawn(async move {
                tokio::select! {
                    // Step 3: Using cloned token to listen to cancellation requests
                    _ = cloned_shutdown_token.cancelled() => {
                        // The token was cancelled, task can shut down
                        let _res = self_clone.trader(&trader_conf, "Close").await;
                    }
                }
            });
            threads.push(t);
        }
        // Cancel the original or cloned token to notify other tasks about shutting down gracefully
        shutdown_token.cancel();
        threads
    }

    /* async fn reload_conf(
        self: Arc<Self>,
        data: DataFrame,
        av: (String, Vec<f64>),
    ) -> Result<Vec<f64>, CLIError> {
        todo!()
    } */

    async fn data_indicator_get(self: Arc<Self>, req: proto::ListNumbersRequest2) -> Vec<f64> {
        let mut c = self.client.clone().unwrap();
        let request = tonic::Request::new(req);
        c.gen_liste(request).await.unwrap().into_inner().result
    }

    //trader for every symbol
    async fn trader(self: Arc<Self>, trader_conf: &TraderConf, col: &str) -> Result<(), CLIError> {
        let self_clone = Arc::clone(&self);
        //get data from csv or grpc
        let df = CsvReadOptions::default()
            .try_into_reader_with_file_path(Some("files/orcl.csv".into()))
            .unwrap()
            .finish()?;

        let close = data_select_column1(df.clone(), col)?;
        let req = ListNumbersRequest2 {
            id: 2, //indicator.into(),
            opt: Some(proto::Opt {
                multiplier: 1.0,
                period: 2,
            }),
            list: close,
        };
        //check with indicator
        //evaluate action
        //execute action

        //ticker
        let trader_conf = trader_conf.clone();
        let t = tokio::spawn(async move {
            loop {
                sleep(std::time::Duration::from_millis(1000)).await;
                let decision = self_clone
                    .clone()
                    .decision_point(&trader_conf, req.clone(), "Close")
                    .await;
            }
        });

        t.await.unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::MockStockActions;
    use crate::types::Action;
    use grpcmock::generate_server;
    use grpcmock::prelude::*;
    use grpcmock::server;
    use polars::prelude::*;
    use proto::*;
    use proto::{indicator_client::IndicatorClient, ListNumbersRequest2, ListNumbersResponse};
    use tonic::transport::Channel;

    // Generate server `MockHelloServer` for the `calculate.Indicator` service.
    generate_server!("calculate.Indicator", MockCalculateServer);

    #[tokio::test]
    async fn new_test() -> Result<(), Box<dyn std::error::Error>> {
        //new TraderConfig
        let tr = TraderConfigs::new("Config.toml", None, "ORCL").await?;
        //get symbol
        let conf_tr = tr.conf_map.get("ORCL");
        //check config exists
        assert!(conf_tr.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn decision_point_test() -> Result<(), Box<dyn std::error::Error>> {
        // Create a new MockSet
        let mut mocks = MockSet::new();
        let price_flag = "Close";

        // Load and insert mocks from mock files
        // NOTE: generic type parameters correspond to prost-generated input and output types of the method.
        let req = ListNumbersRequest2 {
            id: 2,
            opt: Some(proto::Opt {
                multiplier: 1.0,
                period: 2,
            }),
            list: Vec::from([1.0, 2.0, 3.0]),
        };

        mocks.insert(
            GrpcMethod::new("calculate.Indicator", "GenListe")?,
            Mock::new(
                req.clone(),
                ListNumbersResponse {
                    result: Vec::from([1.0, 2.0, 3.0]),
                },
            ),
        );

        // Start mock server
        let server = MockCalculateServer::start(mocks).await?;

        // Create mock client
        let channel = Channel::from_shared(format!("http://0.0.0.0:{}", server.addr().port()))?
            .connect()
            .await?;
        let mut client = IndicatorClient::new(channel);

        //decison_point params
        let indicator = proto::IndicatorType::BollingerBands;
        let sym = String::from("ORCL");

        let mut tr = TraderConfigs::new("Config.toml", Some(client), &sym).await?;
        tr.pull_stock_data().await;
        let arc_tr = Arc::new(tr);
        let tr_conf = TraderConf {
            symbol: sym.clone(),
            price_label: String::from(price_flag),
            indicator: vec![indicator],
        };

        //decison_point test
        let df = DataFrame::default();
        let s = Series::new(price_flag.into(), &[1.0, 2.0, 3.0]);
        let df = DataFrame::new(vec![s]).unwrap();
        let res = arc_tr.decision_point(&tr_conf, req, price_flag).await?;

        assert_eq!(res.action, Action::Buy);
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

    #[tokio::test]
    async fn trader_test() -> Result<(), Box<dyn std::error::Error>> {
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
