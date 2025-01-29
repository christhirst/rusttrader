//#[feature(arbitrary_self_types)]

use mockall::automock;
use polars::{frame::DataFrame, io::SerReader, prelude::CsvReadOptions};
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
    dataframe::data_select_column1,
    error::CLIError,
    helper::desision_maker,
    indicator_decision::action_evaluator,
    proto::{self, indicator_client::IndicatorClient, IndicatorType, ListNumbersRequest2},
    trade::StockActions,
    types::{Action, Indi, TraderConf},
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
    ) -> Result<(), CLIError> {
        //let data = self.clone();
        //let datsa = data_select_column("ORCL", df)?;

        //TODO gen requests
        let close = data_select_column1(df, "Close")?;
        let req = ListNumbersRequest2 {
            id: 2, //indicator.into(),
            opt: Some(proto::Opt {
                multiplier: 1.0,
                period: 2,
            }),
            list: close,
        };

        let indicate = self.clone().grpc(req, conf.symbol.clone()).await;
        //print!("indicate: {:?}", indicate);
        //TODO add dates
        let indicator_selected = vec![proto::IndicatorType::BollingerBands];
        let desc = desision_maker(indicate, indicator_selected);
        let ae = action_evaluator(conf.symbol.clone(), desc);

        match ae.action {
            Action::Buy => self.stock_buy(ae).await,
            Action::Sell => self.stock_sell(ae).await,
            _ => todo!(),
        }
    }
}

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
        let port = conf.grpcport;
        //setup multiple trader
        //benchmark them against each other
        if let Some(client) = client {
            Ok(TraderConfigs {
                conf_map: create_trader,
                client: Some(client),
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

    /* async fn decision_point(
        self: Arc<Self>,
        conf: &TraderConf,
        df: DataFrame,
    ) -> Result<(), CLIError> {
        let data_close = data_select_column1(df, "Close")?;
        let indicate = self
            .clone()
            .grpc(conf.indicator[0], conf.symbol.clone(), data_close)
            .await;

        //TODO add dates
        //let indicator_selected = vec![proto::IndicatorType::BollingerBands];
        let desc = desision_maker(indicate, conf.indicator.clone());
        let ae = action_evaluator(conf.symbol.clone(), desc);
        match ae.action {
            Action::Buy => stock_buy(ae).await,
            Action::Sell => stock_sell(ae).await,
            _ => todo!(),
        }
    } */
    async fn data_indicator_get(self: Arc<Self>, req: proto::ListNumbersRequest2) -> Vec<f64> {
        let mut c = self.client.clone().unwrap();
        let request = tonic::Request::new(req);
        c.gen_liste(request).await.unwrap().into_inner().result
    }

    async fn trader(self: Arc<Self>, trader_conf: &TraderConf) -> Result<(), CLIError> {
        let self_clone = Arc::clone(&self);

        //get data from csv or grpc
        let df = CsvReadOptions::default()
            .try_into_reader_with_file_path(Some("files/orcl.csv".into()))
            .unwrap()
            .finish()?;

        //check with indicator
        //evaluate action
        //execute action
        let _ = self_clone.clone().decision_point(trader_conf, df).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alpaca_to_polars::S;
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

        // Load and insert mocks from mock files
        // NOTE: generic type parameters correspond to prost-generated input and output types of the method.
        mocks.insert(
            GrpcMethod::new("calculate.Indicator", "GenListe")?,
            Mock::new(
                ListNumbersRequest2 {
                    id: 2,
                    opt: Some(proto::Opt {
                        multiplier: 1.0,
                        period: 2,
                    }),
                    list: Vec::from([1.0, 2.0, 3.0]),
                },
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

        //decison_point test
        let df = DataFrame::default();
        let s = Series::new("Close".into(), &[1.0, 2.0, 3.0]);

        let df = DataFrame::new(vec![s]).unwrap();

        let tr = TraderConfigs::new("Config.toml", Some(client), &sym).await?;
        let arc_tr = Arc::new(tr);
        let tr_conf = TraderConf {
            symbol: sym.clone(),
            price_label: String::from("Close"),
            indicator: vec![indicator],
        };
        let arc_mock = arc_tr.decision_point(&tr_conf, df).await;

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
