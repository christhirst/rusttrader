use polars::{frame::DataFrame, io::SerReader, prelude::CsvReadOptions};

use crate::error::CLIError;

pub fn data_csv(filename: String) -> Result<DataFrame, CLIError> {
    let df = CsvReadOptions::default()
        .try_into_reader_with_file_path(Some(filename.into()))
        .unwrap()
        .finish()?;
    Ok(df)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn data_get_test() -> Result<(), Box<dyn std::error::Error>> {
        let df = data_csv(String::from("files/orcl.csv"));
        //println!("{:?}", df);
        assert!(df.is_ok());
        Ok(())
    }
}
