//TODO function with loop
//TODO function either listen to grpc or loop with getting data
//TODO
//TODO
//TODO

#[derive(Clone, Debug)]
pub enum Data_Source {
    Grpc,
    Get,
    Csv,
}

#[derive(Clone, Debug)]
pub struct live_data {
    pub data_source: Data_Source,
}

async fn loop_data(
    ds: Data_Source,
    tc: TraderConfigs,
    conf: &TraderConf,
    req: ListNumbersRequest2,
    col: &str,
) {
    match ds {
        Data_Source::Grpc => {}
        Data_Source::Get => {}
        Data_Source::Csv => loop {
            //TODO get data one by one from csv
            sleep(std::time::Duration::from_millis(1000)).await;
            grpc_listen_or_loop(Data_Source::Csv).await;
        },
    }
    //todo grpc listener
    //grpc or:
}

async fn grpc_listen_or_loop(ds: Data_Source) {
    match ds {
        Data_Source::Grpc => {
            sleep(std::time::Duration::from_millis(1000)).await;
        }
        Data_Source::Get => {}
        Data_Source::Csv => loop_data(todo!(), todo!(), todo!(), todo!(), todo!()).await,
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Action;

    use super::*;

    #[tokio::test]
    async fn stock_buy_test() -> Result<(), Box<dyn std::error::Error>> {
        use mockall::predicate;
        let mut mock = MockStockActions::new();

        Ok(())
    }
}
