//#[feature(arbitrary_self_types)]

use mockall::automock;
use polars::{frame::DataFrame, io::SerReader, prelude::CsvReadOptions};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{error, info};

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
    dataframe::data_select_column1,
    error::CLIError,
    helper::desision_maker,
    indicator_decision::action_evaluator,
    proto::{self, indicator_client::IndicatorClient, ListNumbersRequest2},
    trade::StockActions,
    types::{Action, ActionConfig, ActionEval, ActionValidate, Indi, IndiValidate, TraderConf},
};

#[automock]
trait Calc {
    async fn grpc(
        self: Arc<Self>,
        req: ListNumbersRequest2,
        //ii: IndicatorClient<Channel>,
        symbol: String,
    ) -> Indi;

    async fn decision_point(
        self: Arc<Self>,
        conf: &TraderConf,
        df: DataFrame,
        req: ListNumbersRequest2,
        col: &str,
    ) -> Result<(), CLIError>;
}

impl Calc for TraderConfigs {
    async fn grpc(
        self: Arc<Self>,
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
            println!("test: {:#?}", test);
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
        df: DataFrame,
        req: ListNumbersRequest2,
        col: &str,
    ) -> Result<(), CLIError> {
        let indicate = self.clone().grpc(req, conf.symbol.clone()).await;

        //TODO add dates

        let d = match self.stock_indicators.clone() {
            Some(x) => x,
            None => {
                let mut self_clone = Arc::clone(&self);
                Arc::get_mut(&mut self_clone)
                    .unwrap()
                    .pull_stock_data()
                    .await?
            }
        };
        let u = d.indi_validate.unwrap();

        let desc = desision_maker(indicate, u);
        //TODO

        let ae = action_evaluator(
            conf.symbol.clone(),
            self.stock_indicators
                .clone()
                .unwrap()
                .action_validate
                .unwrap(),
            desc,
        );

        match ae.action {
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
        }
    }
}

#[derive(Clone, Debug)]
pub struct TraderConfigs {
    conf_map: HashMap<String, TraderConf>,
    client: Option<IndicatorClient<Channel>>,
    stock_indicators: Option<ActionConfig>,
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
        //setup multiple trader
        //benchmark them against each other

        if let Some(client) = client {
            Ok(TraderConfigs {
                conf_map: create_trader,
                client: Some(client),
                stock_indicators: None,
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

                self_clone.trader(&trader_conf, "Close").await;
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

    async fn data_indicator_get(self: Arc<Self>, req: proto::ListNumbersRequest2) -> Vec<f64> {
        let mut c = self.client.clone().unwrap();
        let request = tonic::Request::new(req);
        c.gen_liste(request).await.unwrap().into_inner().result
    }

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
        let _ = self_clone
            .clone()
            .decision_point(trader_conf, df, req, "Close")
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::MockStockActions;
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
        let tr = Arc::new(TraderConfigs::new("Config.toml", None, "ORCL").await?);
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
        let arc_mock = arc_tr.decision_point(&tr_conf, df, req, price_flag).await;

        assert!(arc_mock.is_ok());
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
