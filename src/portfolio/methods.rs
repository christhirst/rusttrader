use tracing::{error, info};

use crate::portfolio::types::Portfolio;

impl Portfolio {
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

    pub fn evaluator(
        self,
        a: f32,
        port_ref: &mut Portfolio,
        shares_owned: f64,
        shares_to_buy: f64,
        cash: f64,
        c: f64,
    ) -> (f64, f64) {
        if a >= 1.0 {
            port_ref.buy("ORCL", shares_to_buy, c);
            return (cash, shares_owned); // Buy
        } else if a <= -1.0 {
            port_ref.sell("ORCL", shares_owned, c);
            return (cash, shares_owned); // Sell
        } else {
            (port_ref.cash.unwrap(), shares_owned)
        }
    }
}
