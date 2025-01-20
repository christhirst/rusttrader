use crate::types::{Action, ActionValuator};

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
pub fn action_evaluator(symbol: String, av: Vec<Action>) -> ActionValuator {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_evaluator_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut gg = vec![Action::Buy, Action::Buy, Action::Buy, Action::Sell];
        let sym = String::from("ORCL");
        let action = action_evaluator(sym, gg);
        assert_eq!(action.strength, 0.1);
        Ok(())
    }
}
