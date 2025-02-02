use crate::types::{Action, ActionEval, ActionValidate, ActionValuator};

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
pub fn action_evaluator(symbol: String, eval: ActionValidate, av: Vec<Action>) -> ActionValuator {
    let buy_count = av.iter().filter(|x| **x == Action::Buy).count() as f32;
    let sell_count = av.iter().filter(|x| **x == Action::Sell).count() as f32;
    let eval1 = eval.validate.get(&symbol).unwrap();

    let strength = match eval1 {
        ActionEval::Buy(t) => *t,
        ActionEval::Sell(t) => *t,
        ActionEval::Hold(t) => *t,
    };
    if (buy_count / sell_count) > strength {
        ActionValuator {
            symbol,
            strength: 0.1,
            action: Action::Buy,
        }
    } else if (sell_count / buy_count) > strength {
        ActionValuator {
            symbol,
            strength: 0.2,
            action: Action::Buy,
        }
    } else {
        ActionValuator {
            symbol,
            strength: 0.0,
            action: Action::Hold,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn action_evaluator_test() -> Result<(), Box<dyn std::error::Error>> {
        let gg = vec![Action::Buy, Action::Buy, Action::Buy, Action::Sell];
        let sym = String::from("ORCL");
        let i = ActionValidate {
            validate: HashMap::from([
                (sym.clone(), ActionEval::Buy(0.1)),
                (sym.clone(), ActionEval::Sell(0.2)),
                (sym.clone(), ActionEval::Hold(0.3)),
            ]),
        };
        let action = action_evaluator(sym, i, gg);
        assert_eq!(action.strength, 0.1);
        Ok(())
    }
}
