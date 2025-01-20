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
        s.v.join(&n, ["index"], ["index"], JoinType::Left.into())
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
