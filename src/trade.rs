use apca::{
    api::v2::order::{self, Order, Side, Type},
    ApiInfo, Client,
};
use mockall::automock;
use num_decimal::Num;

use crate::{error::CLIError, trader::TraderConfigs, types::ActionValuator};

#[automock]
pub trait StockActions {
    async fn stock_buy(&self, av: ActionValuator) -> Result<(), CLIError>;
    async fn stock_sell(&self, av: ActionValuator) -> Result<(), CLIError>;
}

impl StockActions for TraderConfigs {
    async fn stock_buy(&self, av: ActionValuator) -> Result<(), CLIError> {
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
        Ok(())
    }

    async fn stock_sell(&self, av: ActionValuator) -> Result<(), CLIError> {
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
        Ok(())
    }
}

pub async fn stock_buy(av: ActionValuator) -> Result<(), CLIError> {
    /* let amount = (av.strength * 10.0) as i64;
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let request = order::CreateReqInit {
        type_: Type::Limit,
        limit_price: Some(Num::from(100)),
        ..Default::default()
    }
    .init(av.symbol, Side::Buy, order::Amount::quantity(amount));

    let order = client.issue::<order::Create>(&request).await?;
    println!("order: {:#?}", order); */
    Ok(())
}
pub async fn stock_sell(av: ActionValuator) -> Result<(), CLIError> {
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
    Ok(())
}

/* async fn data_get(date: &str) -> Result<apca::data::v2::bars::Bars, CLIError> {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = DateTime::from_str(date).unwrap();
    let end = start.checked_add_months(Months::new(2)).unwrap();
    let request = ListReqInit {
        limit: Some(5),
        ..Default::default()
    }
    .init("AAPL", start, end, TimeFrame::OneDay);
    let e = client.issue::<List>(&request).await?;
    Ok(e)
} */

#[cfg(test)]
mod tests {
    use crate::types::Action;

    use super::*;

    #[tokio::test]
    async fn stock_buy_test() -> Result<(), Box<dyn std::error::Error>> {
        use mockall::predicate;
        let mut mock = MockStockActions::new();

        /* let tc = TraderClient {};
        let aeval = ActionValuator {
            symbol: "AAPL".to_string(),
            strength: 0.5,
            action: Action::Buy,
        };
        StockActions::stock_buy(&tc, aeval).await?; */

        /*
        let order = stock_buy(ActionValuator {
            symbol: "AAPL".to_string(),
            strength: 0.5,
            action: Action::Buy,
        })
        .await?; */

        //assert_eq!(order, ());
        Ok(())
    }
}
