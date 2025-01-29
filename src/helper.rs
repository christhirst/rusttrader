use crate::{proto, types::Action, types::Indi};

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

//evaluate all indicators
pub fn desision_maker(indicator: Indi, indicator_select: Vec<proto::IndicatorType>) -> Vec<Action> {
    let mut action = vec![];

    for i in indicator_select {
        match indicator.indicator.get(&i) {
            Some(x) => {
                if *x > 0.1 {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[tokio::test]
    async fn desision_maker_test() -> Result<(), Box<dyn std::error::Error>> {
        let mut indicator_list = HashMap::from([(proto::IndicatorType::BollingerBands, 0.1)]);
        let hm = Indi {
            symbol: String::from("ORCL"),
            indicator: indicator_list,
        };
        let indicator_selected = vec![proto::IndicatorType::BollingerBands];
        let handles = desision_maker(hm, indicator_selected);
        assert_eq!(handles, vec![Action::Sell]);
        Ok(())
    }
}
