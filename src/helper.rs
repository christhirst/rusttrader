use crate::{
    proto,
    types::{Action, Indi, IndiValidate},
};

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

//Evaluate all indicators
//Indicator are the values to evaluate with indicagor_val
pub fn desision_maker(indicator_values: Indi, indicator_eval: IndiValidate) -> Vec<Action> {
    let mut action = vec![];

    if let Some(eval) = indicator_eval.validate.get(&indicator_values.symbol) {
        for i in indicator_values.indicator.iter() {
            match indicator_values.indicator.get(i.0) {
                Some(x) => {
                    let o = eval.get(i.0).unwrap();
                    if *x > *o {
                        action.push(Action::Buy)
                    } else {
                        action.push(Action::Sell)
                    }
                }
                None => action.push(Action::Hold),
            };
        }
    }

    action
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
        let indicator_selected = IndiValidate {
            validate: HashMap::from([(
                sym.clone(),
                HashMap::from([(proto::IndicatorType::BollingerBands, 0.1)]),
            )]),
        };
        let handles = desision_maker(hm.clone(), indicator_selected);
        assert_eq!(handles, vec![Action::Sell]);

        let indicator_selected = IndiValidate {
            validate: HashMap::from([(
                sym,
                HashMap::from([(proto::IndicatorType::BollingerBands, 0.01)]),
            )]),
        };
        let handles = desision_maker(hm, indicator_selected);
        assert_eq!(handles, vec![Action::Buy]);

        Ok(())
    }
}
