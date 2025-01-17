#[feature(arbitrary_self_types)]
use apca::{
    api::v2::order::{self, Order, Side, Type},
    ApiInfo, Client,
};
use num_decimal::Num;
use polars::{
    io::SerReader,
    prelude::{col, CsvReadOptions, DataFrame, IntoLazy},
};
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
    error::CLIError,
    proto::{self, indicator_client::IndicatorClient, IndicatorType},
};

#[derive(Clone)]
pub struct TraderConf {
    symbol: String,
    indicator: Vec<proto::IndicatorType>,
}

/* struct Trade {
    date: String,
    Action: Action,
} */

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
        let token = CancellationToken::new();

        let arc = Arc::new(self);
        let mut treads: Vec<JoinHandle<()>> = vec![];
        for (_symbol, trader_conf) in arc.conf_map.clone() {
            let cloned_token = token.clone();
            let self_clone = Arc::clone(&arc);
            // No need to clone if not using Arc
            //let client = (&self.trader).clone(); // Use `clone()` if `IndicatorClient` implements Clone
            let t = tokio::spawn(async move {
                tokio::select! {
                    // Step 3: Using cloned token to listen to cancellation requests
                    _ = cloned_token.cancelled() => {
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
            // Step 4: Cancel the original or cloned token to notify other tasks about shutting down gracefully
            token.cancel();
        }
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

    //
    /* async fn data_indicator_get(self: Arc<Self>, req: proto::ListNumbersRequest2) -> Vec<f64> {
        let mut c = self.client.clone().unwrap();
        let request = tonic::Request::new(req);
        c.gen_liste(request).await.unwrap().into_inner().result
    } */

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

    async fn trader(self: Arc<Self>, conf: &TraderConf) {
        let self_clone = Arc::clone(&self);
        for i in conf.indicator.iter() {
            let _ = self_clone.clone().decision_point(*i).await;
        }
    }
}

//TODO decisions as funcions
#[tracing::instrument]
fn decision_maker_vec(indicator: Vec<f64>) -> Vec<u32> {
    let actions_vec: Vec<u32> = indicator
        .iter()
        .map(|x| {
            if *x > 2.14 {
                Action::Buy as u32
            } else {
                Action::Sell as u32
            }
        })
        .collect();

    actions_vec
}

/* fn decision_bollinger_bands(upperlower: Vec<(f64, f64, f64)>) -> Vec<u32> {
    let actions_vec: Vec<u32> = upperlower
        .iter()
        .map(|(u, _m, _l)| {
            if *u > 2.14 {
                Action::Buy as u32
            } else {
                Action::Sell as u32
            }
        })
        .collect();

    actions_vec
} */

//evaluate all indicators
fn desision_maker(indicator: Indi, indicator_select: Vec<proto::IndicatorType>) -> Vec<Action> {
    let mut action = vec![];

    for i in indicator_select {
        match indicator.indicator.get(&i) {
            Some(x) => {
                if *x > 0.1 {
                    action.push(Action::Buy)
                } else {
                    action.push(Action::Sell)
                }
            }
            None => action.push(Action::Hold),
        };
    }
    action
}

#[derive(Clone, Debug)]
struct Indi {
    symbol: String,
    indicator: HashMap<proto::IndicatorType, f64>,
}

fn data_select_column(column: &str, df: DataFrame) -> Result<DataFrame, CLIError> {
    //df.with_column(column("Close").cast::<Float64>())
    //Date,Open,High,Low,Close,Adj Close,Volume
    let result = df
        .clone()
        .lazy()
        .select([col("Date"), col(column)])
        .collect()?;
    //let close: Vec<f64> = df["Close"].f64().unwrap().to_vec_null_aware().unwrap_left();
    Ok(result)
}

fn action_evaluator(symbol: String, av: Vec<Action>) -> ActionValuator {
    let buy_count = av.iter().filter(|x| **x == Action::Buy).count();
    let sell_count = av.iter().filter(|x| **x == Action::Sell).count();
    if buy_count > sell_count * 2 {
        ActionValuator {
            symbol: String::from(symbol),
            strength: 0.1,
            action: Action::Buy,
        }
    } else {
        ActionValuator {
            symbol: String::from(symbol),
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

/* fn data_csv(filename: String) -> Result<DataFrame, CLIError> {
    //"files/orcl.csv".into()
    let df = CsvReadOptions::default()
        .try_into_reader_with_file_path(Some(filename.into()))
        .unwrap()
        .finish()?;
    Ok(df)
    // let s0 = Series::new(av., av.values().cloned().collect::<Vec<f64>>());
} */

/* fn data_filter(df: DataFrame) -> Result<DataFrame, CLIError> {
    let filtered_df = df
        .lazy()
        .filter(col("Action").eq(Action::Sell as u32))
        .collect()?;
    Ok(filtered_df)
} */

/* fn data_join(df: DataFrame, df1: DataFrame) -> Result<DataFrame, CLIError> {
    // In Rust, we cannot use the shorthand of specifying a common
    // column name just once.
    let result = df
        .clone()
        .lazy()
        .join(
            df1.lazy(),
            [col("Date")],
            [col("Date")],
            JoinArgs::default(),
        )
        .collect()?;
    println!("{}", result);
    Ok(result)
} */

/* fn data_append(mut df: DataFrame, av: (String, Vec<f64>)) -> Result<DataFrame, CLIError> {
    let i = df.with_column(Series::new(av.0.into(), av.1)).cloned()?;
    Ok(i)
    // let s0 = Series::new(av., av.values().cloned().collect::<Vec<f64>>());
} */

/* fn data_append2(mut df: DataFrame, av: (String, Vec<u32>)) -> Result<DataFrame, CLIError> {
    let i = df.with_column(Series::new(av.0.into(), av.1)).cloned()?;
    Ok(i)
    // let s0 = Series::new(av., av.values().cloned().collect::<Vec<f64>>());
} */

/// Appends two DataFrames together horizontally.   
/* async fn df_append(mut df: DataFrame, add: Series) -> Result<DataFrame, CLIError> {
    let i = df.with_column(add).cloned()?;
    Ok(i)
    // let s0 = Series::new(av., av.values().cloned().collect::<Vec<f64>>());
}*/

/* async fn df_to_vec(df: DataFrame, column: &str) -> Result<Vec<f64>, CLIError> {
    let close = df[column]
        .f64()
        .unwrap()
        .to_vec_null_aware()
        .left()
        .ok_or(CLIError::ConvertingError);
    close
    // let s0 = Series::new(av., av.values().cloned().collect::<Vec<f64>>());
} */

fn data_select_column1(df: DataFrame, column: &str) -> Result<Vec<f64>, CLIError> {
    /* let df = CsvReadOptions::default()
    .try_into_reader_with_file_path(Some("files/orcl.csv".into()))
    .unwrap()
    .finish()?; */
    //df.with_column(column("Close").cast::<Float64>())
    //Date,Open,High,Low,Close,Adj Close,Volume

    let close: Vec<f64> = df[column].f64().unwrap().to_vec_null_aware().unwrap_left();
    Ok(close)
}

#[derive(Clone, PartialEq, Debug)]
pub enum Action {
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
        let mut indicator_list = HashMap::from([(proto::IndicatorType::BollingerBands, 0.1)]);
        let hm = Indi {
            symbol: String::from("ORCL"),
            indicator: indicator_list,
        };
        let indicator_selected = vec![proto::IndicatorType::BollingerBands];
        let handles = desision_maker(hm, indicator_selected);
        assert_eq!(handles, vec![Action::Sell]);
        Ok(())
    }

    #[test]
    fn action_evaluator_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut gg = vec![Action::Buy, Action::Buy, Action::Buy, Action::Sell];
        let sym = String::from("ORCL");
        action_evaluator(sym, gg);
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

    /*
    #[tokio::test]
     async fn data_get_append_test() -> Result<(), Box<dyn std::error::Error>> {
        //let data = data_csv(String::from("files/orcl.csv")).unwrap();
        let df = data_csv(String::from("files/orcl.csv")).unwrap();
        let tr = TraderConfigs::new("Config.toml").await?;
        let foo = Arc::new(tr);

        let data = df_to_vec(df.clone(), "Close").await?;
        let req = proto::ListNumbersRequest2 {
            id: IndicatorType::BollingerBands.into(),
            opt: None,
            list: data,
        };
        let oo = foo.clone().data_indicator_get(req).await;
        let ii = (String::from("BOL"), oo);
        let oo = data_append(df, ii);
        println!("{:?}", oo.unwrap().head(Some(3)));
        Ok(())
    } */
    /*  #[tokio::test]
    async fn data_append_indicator_test() -> Result<(), Box<dyn std::error::Error>> {
        //let data = data_csv(String::from("files/orcl.csv")).unwrap();
        let mut df = data_csv(String::from("files/orcl.csv")).unwrap();
        df = df.drop("Open").unwrap();
        df = df.drop("High").unwrap();
        df = df.drop("Low").unwrap();
        df = df.drop("Adj Close").unwrap();
        let tr = TraderConfigs::new("Config.toml").await?;
        let foo = Arc::new(tr);
        let data = df_to_vec(df.clone(), "Close").await?;

        let opt = proto::Opt {
            multiplier: 2.0,
            period: 5,
        };
        let req = proto::ListNumbersRequest2 {
            id: IndicatorType::BollingerBands.into(),
            opt: Some(opt),
            list: data,
        };

        //get Indicator values as Vec<f64>
        let oo = foo.clone().data_indicator_get(req).await;

        let ii = (String::from("BOL"), oo);
        //append Indicator data
        let oo = data_append(df.clone(), ii)?;

        //get Indicator values as Vec<f64>
        let data = df_to_vec(oo.clone(), "Close").await?;

        //get action values as Vec<u32>
        let actions = decision_maker_vec(data);

        let ii = (String::from("Action"), actions);
        //append Indicator data
        let iw = data_append2(oo, ii)?;
        println!("{:?}", iw.tail(Some(3)));

        /* let actions = desision_maker();
        data_append(df); */

        //let iis = (String::from("ACTION"), oo);
        //let oo = data_select_column("Close");
        let zz = data_filter(iw)?;
        println!("{:?}", zz.tail(Some(3)));
        Ok(())
    } */
}

//PUT/CALL ratio, VIX, AAII Sentiment, Fear and Greed Index
