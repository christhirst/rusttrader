use std::collections::HashMap;

use crate::{
    proto,
    types::{Action, Indi, IndiValidate},
};

//Evaluate all indicators
//Indicator are the values to evaluate with indicagor_val
pub fn desision_maker(
    indicator_values: Indi,
    indicator_eval: HashMap<proto::IndicatorType, f64>,
) -> Vec<Action> {
    let mut action = vec![];

    for i in indicator_values.indicator.iter() {
        match indicator_values.indicator.get(i.0) {
            Some(x) => {
                let o = indicator_eval.get(i.0).unwrap();
                if *x > *o {
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

fn amount(funds: f64, fraction: f64, price: f64) -> i64 {
    ((funds * fraction) / price) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn desision_maker_test() -> Result<(), Box<dyn std::error::Error>> {
        let sym = String::from("ORCL");
        let mut indicator_list = HashMap::from([(proto::IndicatorType::BollingerBands, 0.1)]);
        let hm = Indi {
            symbol: sym.clone(),
            indicator: indicator_list,
        };
        let indicator_selected = HashMap::from([(proto::IndicatorType::BollingerBands, 0.1)]);
        let handles = desision_maker(hm.clone(), indicator_selected.clone());
        assert_eq!(handles, vec![Action::Sell]);

        let handles = desision_maker(hm, indicator_selected);
        assert_eq!(handles, vec![Action::Sell]);

        Ok(())
    }
}
