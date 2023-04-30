use crate::{CardCount, StateArray};

pub fn gather_hand_count_states<F, G>(
    charlie_number: u8,
    feature_fn: F,
    state_filter: G,
) -> Vec<Vec<CardCount>>
where
    F: Fn(&CardCount) -> usize,
    G: Fn(&CardCount) -> bool,
{
    let mut ret = Vec::new();
    let mut card_count = CardCount::with_number_of_decks(0);
    gather_hand_count_states_aux(
        &charlie_number,
        &feature_fn,
        &state_filter,
        &mut card_count,
        1,
        &mut ret,
    );
    ret
}

fn gather_hand_count_states_aux<F, G>(
    charlie_number: &u8,
    feature_fn: &F,
    state_filter: &G,
    current_card_count: &mut CardCount,
    loop_start_card: u8,

    result: &mut Vec<Vec<CardCount>>,
) where
    F: Fn(&CardCount) -> usize,
    G: Fn(&CardCount) -> bool,
{
    if state_filter(current_card_count) {
        let feature = feature_fn(current_card_count);
        while result.len() <= feature {
            result.push(vec![]);
        }
        result[feature].push(*current_card_count);
    }

    if current_card_count.get_sum() >= 21
        || current_card_count.get_total() == *charlie_number as u16
    {
        return;
    }

    for i in loop_start_card..=10 {
        current_card_count.add_card(i);
        gather_hand_count_states_aux(
            charlie_number,
            feature_fn,
            state_filter,
            current_card_count,
            i,
            result,
        );
        current_card_count.remove_card(i);
    }
}

pub fn gather_dealer_count_states<F>(
    dealer_hit_on_soft17: bool,
    feature_fn: F,
) -> Vec<Vec<CardCount>>
where
    F: Fn(&CardCount) -> usize,
{
    let mut ret = Vec::new();
    let mut card_count = CardCount::with_number_of_decks(0);
    let mut is_visited = StateArray::new();
    gather_dealer_count_states_aux(
        &dealer_hit_on_soft17,
        &feature_fn,
        &mut card_count,
        &mut is_visited,
        &mut ret,
    );
    ret
}

fn gather_dealer_count_states_aux<F>(
    dealer_hit_on_soft17: &bool,
    feature_fn: &F,
    current_card_count: &mut CardCount,
    is_visited: &mut StateArray<()>,

    result: &mut Vec<Vec<CardCount>>,
) where
    F: Fn(&CardCount) -> usize,
{
    if is_visited.contains_state(current_card_count) {
        return;
    }
    is_visited[current_card_count] = ();

    let feature = feature_fn(current_card_count);
    while result.len() <= feature {
        result.push(vec![]);
    }
    result[feature].push(*current_card_count);

    let must_stand = {
        let actual_sum = current_card_count.get_actual_sum();
        let is_soft = current_card_count.is_soft();
        if actual_sum > 17 {
            true
        } else if actual_sum < 17 {
            false
        } else {
            if !is_soft {
                true
            } else {
                !dealer_hit_on_soft17
            }
        }
    };

    if must_stand {
        return;
    }

    for i in 1..=10 {
        current_card_count.add_card(i);
        gather_dealer_count_states_aux(
            dealer_hit_on_soft17,
            feature_fn,
            current_card_count,
            is_visited,
            result,
        );
        current_card_count.remove_card(i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn get_number_of_hand_states() {
        let charlie_number: u8 = 6;
        let f = |card_count: &CardCount| card_count.get_sum() as usize;
        let g = |card_count: &CardCount| card_count.get_total() <= 6;
        let gathered_states = gather_hand_count_states(charlie_number, f, g);
        let mut acc = 0;
        for (i, states) in gathered_states.iter().enumerate() {
            acc += states.len();
            println!("({}, {}, {})", i, states.len(), acc);
        }
    }

    #[test]
    #[ignore]
    fn get_number_of_dealer_states() {
        let dealer_hit_on_soft17 = false;
        let f = |card_count: &CardCount| card_count.get_actual_sum() as usize;
        let gathered_states = gather_dealer_count_states(dealer_hit_on_soft17, f);
        let mut acc = 0;
        for (i, states) in gathered_states.iter().enumerate() {
            acc += states.len();
            println!("({}, {}, {})", i, states.len(), acc);
        }
    }
}
