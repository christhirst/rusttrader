//#[feature(arbitrary_self_types)]

use apca::data::v2::stream::Bar;
use chrono::{DateTime, Duration, TimeZone, Utc};
use mockall::automock;
use num_decimal::Num;
use polars::{
    frame::DataFrame,
    io::SerReader,
    prelude::{
        col, ChunkedArray, CsvReadOptions, DatetimeType, Float64Type, Int32Type, Int64Type,
        IntoLazy, Logical, NamedFrom,
    },
    series::Series,
};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{error, info};

use std::{
    collections::{HashMap, VecDeque},
    fmt::{self},
    str::FromStr,
    sync::{Arc, Mutex},
    vec,
};
use tokio::{task::JoinHandle, time::sleep};
use tonic::transport::Channel;

use crate::{
    config::AppConfig,
    config2::Settings,
    data::data_csv,
    dataframe::data_select_column1,
    error::CLIError,
    helper::desision_maker,
    indicator_decision::action_evaluator,
    proto::{self, indicator_client::IndicatorClient, ListNumbersRequest2},
    trade::{self, StockActions},
    types::{
        ActionConfig, ActionEval, ActionValidate, ActionValuator, Buffer, Indi, IndiValidate,
        TraderConf,
    },
};

#[automock]
trait Calc {
    async fn grpc(&self, req: ListNumbersRequest2, symbol: String) -> Indi;
    async fn decision_point(
        self: Arc<Self>,
        conf: &TraderConf,
        req: ListNumbersRequest2,
        col: &str,
    ) -> Result<ActionValuator, CLIError>;
    //fn makes_adder(self: Arc<Self>, d: DateTime<Utc>, o: f64, c: f64, h: f64, l: f64) -> f64;
}

#[automock]
pub trait PortfolioActions {
    fn buy(&mut self, symbol: &str, share_amount: f64, share_price: f64) {}
    fn sell(&mut self, symbol: &str, share_amount: f64, share_price: f64) {}
    fn evaluator(
        self,
        port_ref: &mut Portfolio,
        shares_owned: f64,
        shares_to_buy: f64,
        cash: f64,
        a: i32,
        c: f64,
    ) -> (f64, f64);
}

#[derive(Clone, Debug)]
pub struct Portfolio {
    pub name: String,
    pub cash: Option<f64>,
    pub stocks: Option<HashMap<String, f64>>, // symbol and amount of shares
}

impl PortfolioActions for Portfolio {
    fn buy(&mut self, symbol: &str, share_amount: f64, share_price: f64) {
        info!("Buying {} shares of {}", share_amount, symbol);
        if self.cash.is_none() || self.cash.unwrap() < share_amount * share_price {
            error!("Not enough cash to buy shares");
            return;
        }
        self.cash = Some(self.cash.unwrap() - share_amount * share_price);
        self.stocks
            .as_mut()
            .unwrap()
            .entry("ORCL".to_string())
            .and_modify(|value| *value += share_amount);
    }

    fn sell(&mut self, symbol: &str, share_amount: f64, share_price: f64) {
        info!("Selling {} shares of {}", share_amount, symbol);
        if *self.stocks.clone().unwrap().get(symbol).unwrap() < share_amount {
            error!("Not enough cash to buy shares");
            return;
        }
        self.cash = Some(self.cash.unwrap() + share_amount * share_price);
        self.stocks
            .as_mut()
            .unwrap()
            .entry("ORCL".to_string())
            .and_modify(|value| *value -= share_amount);
    }
    fn evaluator(
        self,
        port_ref: &mut Portfolio,
        shares_owned: f64,
        shares_to_buy: f64,
        cash: f64,
        a: i32,
        c: f64,
    ) -> (f64, f64) {
        if a == 1 {
            port_ref.buy("ORCL", shares_to_buy, c);
            return (cash, shares_owned); // Buy
        } else if a == -1 {
            port_ref.sell("ORCL", shares_to_buy, c);
            return (cash, shares_owned); // Sell
        } else {
            (port_ref.cash.unwrap(), shares_owned)
        }
    }
}

#[derive(Clone, Debug)]
pub struct TraderConfigs {
    conf_map: HashMap<String, Vec<TraderConf>>,
    portfolio: Option<Portfolio>,
    client: Option<IndicatorClient<Channel>>,
    stock_indicators: Option<ActionConfig>,
}

fn EvaluatorCompair(bar_Buffer: &VecDeque<Bar>) -> i32 {
    let res = if bar_Buffer[0].close_price > bar_Buffer[4].close_price {
        1
    } else {
        -1
    };
    res
}

fn BufferEvaluate(
    tc: &mut TraderConf,
    /* buffer: &mut VecDeque<Bar>,
    buffer_capacity: usize, */
    bar_new: Bar,
) -> i32 {
    //TODO Read Conf for Indicator
    //MATCH to IndicatorType
    //let buffer_capacity = tc.buff.capacity;

    let res = if tc.buff.data.len() == tc.buff.capacity {
        let res = EvaluatorCompair(&tc.buff.data);
        let buffer_from_self = &mut tc.buff.data;
        let poped = buffer_from_self.pop_front();
        /* let res = if poped.unwrap().close_price > bar_new.close_price {
            1
        } else {
            -1
        }; */
        res
    } else {
        0
    };
    &mut tc.buff.data.push_back(bar_new);
    res
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
    //TODO udjust to new structure
    async fn decision_point(
        self: Arc<Self>,
        conf: &TraderConf,
        req: ListNumbersRequest2,
        col: &str,
    ) -> Result<ActionValuator, CLIError> {
        let indicator_from_stock_data = self.clone().grpc(req, conf.symbol.clone()).await;
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

impl fmt::Debug for TraderConf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "TraderConf {{ symbol: {}, indicator: {:?} }}",
            self.symbol, self.indicator
        )
    }
}

pub fn actionTest(d: DateTime<Utc>, a: i32, c: f64) -> (f64, f64) {
    (2.0, 2.0)
}

fn zipper(
    s: &mut TraderConfigs,
    d: &Logical<DatetimeType, Int64Type>,
    a: &ChunkedArray<Int32Type>,
    c: &ChunkedArray<Float64Type>,
) -> Vec<(f64, f64)> {
    let values: Vec<(f64, f64)> = c
        .into_iter()
        .zip(a.into_iter())
        .zip(d.into_iter())
        .map(|(((opt_c), opt_a), opt_d)| match (opt_d, opt_a, opt_c) {
            (Some(d), Some(a), Some(c)) => s.actionTest(Utc.timestamp_opt(d, 0).unwrap(), a, c),
            _ => (0.0, 0.0),
        })
        .collect();
    values
}

impl TraderConfigs {
    pub async fn new(
        settings: Settings,
        path: &str,
        client: Option<IndicatorClient<Channel>>,
        sym: &str,
    ) -> Result<Self, CLIError> {
        let mut create_trader = HashMap::new();
        for (i, c) in settings.Stockconfig.iter() {
            let tc = TraderConf {
                symbol: c.symbol.clone(),
                price_label: String::from("Close"),
                indicator: c.indicator.clone(),
                shares_to_buy: c.shares_to_buy,
                buff: Buffer {
                    capacity: c.buffersize,
                    data: VecDeque::with_capacity(c.buffersize),
                },
            };
            create_trader.insert(sym.to_string(), vec![tc]);
        }

        let _port = settings.grpc.grpcport.clone();
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

        if let client = client {
            Ok(TraderConfigs {
                conf_map: create_trader,
                portfolio: Some(Portfolio {
                    name: String::from("Default Portfolio"),
                    cash: Some(1000.0),
                    stocks: Some(HashMap::from([("ORCL".to_string(), 0.0)])),
                }),
                client: None,
                stock_indicators: Some(ac),
            })
        } else {
            Err(CLIError::Converting)
        }
    }

    //TODO holding shares
    //series to graph

    pub fn actionTest(&mut self, d: DateTime<Utc>, a: i32, c: f64) -> (f64, f64) {
        let port_ref = self.portfolio.as_mut().unwrap();
        let shares_to_buy = 20.0;
        let mut cash = port_ref.cash.unwrap();
        let mut shares_owned = port_ref
            .stocks
            .clone()
            .unwrap()
            .get("ORCL")
            .unwrap()
            .clone();
        port_ref
            .clone()
            .evaluator(port_ref, shares_owned, 20.0, cash, a, c)
    }

    pub fn actionEval(&mut self, mut df: DataFrame) -> DataFrame {
        let oo = &self.conf_map.clone();
        for (symbol, trader_conf) in oo {
            println!("Symbol: {}", symbol);

            //let oo = self.conf_map.get_mut("ORCL").unwrap().clone();
            for i in trader_conf {
                let date = df.column("Date").unwrap().datetime().unwrap();
                let action = df.column("Action").unwrap().i32().unwrap();
                let close = df.column("Close").unwrap().f64().unwrap();

                // Collect the zipped data first
                let zipped: Vec<(Option<i64>, Option<i32>, Option<f64>)> = date
                    .into_iter()
                    .zip(action.into_iter())
                    .zip(close.into_iter())
                    .map(|((d, a), c)| (d, a, c))
                    .collect();

                // Now you can call self.actionTest one at a time, avoiding nested borrows
                let values: Vec<(f64, f64)> = zipped
                    .into_iter()
                    .map(|(opt_d, opt_a, opt_c)| match (opt_d, opt_a, opt_c) {
                        (Some(d), Some(a), Some(c)) => {
                            self.actionTest(Utc.timestamp_opt(d, 0).unwrap(), a, c)
                        }
                        _ => (0.0, 0.0),
                    })
                    .collect();

                let (vec1, vec2): (Vec<f64>, Vec<f64>) = values.into_iter().unzip();
                let new_series1 = Series::new("Portfolio".into(), vec1);
                let new_series2 = Series::new(i.symbol.clone().into(), vec2);
                df.with_column(new_series1).unwrap();
                df.with_column(new_series2).unwrap();
            }
        }
        //println!("{:?}", df.lazy().filter(col("ORCL").gt(1.0)).collect());
        //println!("{:?}", df.lazy().filter(col("ORCL").gt(1.0)).collect());
        println!("{:?}", df);
        df
        //todo!()
    }

    pub fn traders(&mut self, d: DateTime<Utc>, sym: &str, o: f64, c: f64, h: f64, l: f64) -> i32 {
        // conf_map: HashMap<String, TraderConf>
        //let buffer_capacity = self.conf_map.get_mut(sym).unwrap().buff.capacity;
        //let buffer_from_self = &mut self.conf_map.get_mut(sym).unwrap().buff.data;
        let bar_new = Bar {
            symbol: sym.to_string(),
            open_price: Num::from_str(&o.to_string()).unwrap(),
            high_price: Num::from_str(&h.to_string()).unwrap(),
            low_price: Num::from_str(&l.to_string()).unwrap(),
            close_price: Num::from_str(&c.to_string()).unwrap(),
            volume: Num::from(100),
            timestamp: d,
        };
        let oo = self.conf_map.get_mut(sym).unwrap();
        let oo = oo.first_mut().unwrap();
        BufferEvaluate(oo, bar_new)
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
            let trader_conf = trader_conf.first().unwrap().clone();
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

    //DATA FAKE
    async fn data_from_csv(&mut self) -> DataFrame {
        let mut df = data_csv(String::from("files/orcl.csv")).unwrap();
        //timestamp: DateTime<Utc>
        // Parse the "date" column as Utf8 (string), then convert to DateTime<Utc>
        let sym = "ORCL";

        let date = df.column("Date").unwrap().datetime().unwrap();
        let open = df.column("Open").unwrap().f64().unwrap();
        let close = df.column("Close").unwrap().f64().unwrap();
        let high = df.column("High").unwrap().f64().unwrap();
        let low = df.column("Low").unwrap().f64().unwrap();
        //let Volume = df.column("Volume").unwrap().f64().unwrap();

        //TODO Each SYML gets a Config with inducators
        //TODO data from csv in config or stream
        // Perform the operation
        let values: Vec<i32> = close
            .into_iter()
            .zip(open.into_iter())
            .zip(high.into_iter())
            .zip(low.into_iter())
            .zip(date.into_iter())
            .map(|((((opt_c, opt_l), opt_h), opt_o), opt_d)| {
                match (opt_d, opt_l, opt_h, opt_o, opt_c) {
                    (Some(d), Some(o), Some(h), Some(l), Some(c)) => {
                        self.traders(Utc.timestamp_opt(d, 0).unwrap(), sym, o, c, h, l)
                    }
                    _ => 0,
                }
            })
            .collect();

        let new_series = Series::new("Action".into(), values);
        df.with_column(new_series).unwrap();
        df = df.drop("Open").unwrap();
        df = df.drop("High").unwrap();
        df = df.drop("Low").unwrap();
        df = df.drop("Adj Close").unwrap();
        df = df.drop("Volume").unwrap();

        println!("{:?}", df);
        df
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
    use crate::Settings;
    /* use grpcmock::generate_server;
    use grpcmock::prelude::*;
    use grpcmock::server; */
    use polars::prelude::*;
    use proto::*;
    use proto::{indicator_client::IndicatorClient, ListNumbersRequest2, ListNumbersResponse};
    use tonic::transport::Channel;

    #[tokio::test]
    async fn data_csv_test() {
        //let client = IndicatorClient::connect("http://[::1]:50051").await?;

        let settings = Settings::new().unwrap();
        println!("{settings:?}");

        let mut tr = TraderConfigs::new(settings, "Config.toml", None, "ORCL")
            .await
            .unwrap();

        let action_vec = tr.data_from_csv().await;
        tr.actionEval(action_vec);

        panic!("Test not implemented yet");
    }

    #[tokio::test]
    async fn portfolio_read_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut portfolio = Portfolio {
            name: String::from("Test Portfolio"),
            cash: Some(1000.0),
            stocks: Some(HashMap::from([("ORCL".to_string(), 0.0)])),
        };
        portfolio.buy("ORCL", 10.0, 50.0); //500 10
        assert_eq!(portfolio.cash.unwrap(), 500.0);
        portfolio.sell("ORCL", 5.0, 55.0); //775 5
        println!("Portfolio: {:?}", portfolio);
        assert_eq!(portfolio.cash.unwrap(), 775.0);
        assert_eq!(portfolio.stocks.unwrap().get("ORCL").unwrap(), &5.0);
        assert_eq!(portfolio.name, "Test Portfolio");

        Ok(())
    }

    /* #[tokio::test]
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
    async fn data_read() -> Result<(), Box<dyn std::error::Error>> {
        let df = CsvReadOptions::default()
            .try_into_reader_with_file_path(Some("files/orcl.csv".into()))
            .unwrap()
            .finish()?;
        println!("symbol: {:?}", df);
        Ok(())
    }

    #[tokio::test]
    async fn trader_test() -> Result<(), Box<dyn std::error::Error>> {
        //let tr = TraderConfigs::new("Config.toml").await?;
        /* let foo = Arc::new(tr);
        let req = proto::ListNumbersRequest2 {
            id: indicator.into(),
            list: data,
        };

        foo.data_indicator_get(req); */
        /* let client = IndicatorClient::connect("http://[::1]:50051").await?;
        let tr = TraderConfigs::new("Config.toml", Some(client), "ORCL").await?;

        let settings = Settings::new().unwrap();

        for i in settings.symbols {
            println!("symbol: {:?}", i);
        }

        let tr_config = Arc::new(Mutex::new(tr.clone()));

        //spawn trader
        let handles = tr.trader_spawn(tr_config).await;

        for i in handles {
            i.await.unwrap();
        } */

        Ok(())
    }
}

//PUT/CALL ratio, VIX, AAII Sentiment, Fear and Greed Index
