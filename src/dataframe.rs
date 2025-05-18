use polars::{
    frame::DataFrame,
    prelude::{col, DataFrameJoinOps, IntoLazy, JoinType},
    time::DynamicGroupOptions,
};

use crate::{alpaca_to_polars::S, error::CLIError};

pub fn data_select_column(column: &str, df: DataFrame) -> Result<DataFrame, CLIError> {
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

fn data(res: apca::data::v2::bars::Bars, span: DynamicGroupOptions) -> Result<DataFrame, CLIError> {
    let bars = res.bars;
    let mut s: S = bars.into();
    s.v = s.v.with_row_index("index".into(), None).unwrap();
    let n =
        s.v.clone()
            .lazy()
            .select([col("*")])
            .group_by_dynamic(col("time"), [], span)
            .agg([col("close").mean().alias("name")])
            .collect()
            .unwrap()
            .with_row_index("index".into(), None)
            .unwrap();
    let joined =
        s.v.join(&n, ["index"], ["index"], JoinType::Left.into(), None)
            .unwrap();
    let oo = joined
        .clone()
        .lazy()
        .select([col("*").exclude(["time_right"])])
        .collect()
        .unwrap();
    Ok(oo)
}

pub fn data_select_column1(df: DataFrame, column: &str) -> Result<Vec<f64>, CLIError> {
    /* let df = CsvReadOptions::default()
    .try_into_reader_with_file_path(Some("files/orcl.csv".into()))
    .unwrap()
    .finish()?; */
    //df.with_column(column("Close").cast::<Float64>())
    //Date,Open,High,Low,Close,Adj Close,Volume

    let close: Vec<f64> = df[column].f64().unwrap().to_vec_null_aware().unwrap_left();
    Ok(close)
}

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

// Appends two DataFrames together horizontally.
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
