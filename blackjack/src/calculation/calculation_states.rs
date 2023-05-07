use crate::{CardCount, SingleStateArray};

#[derive(Debug, Clone)]
pub struct HandShoePair {
    pub hand: CardCount,
    pub shoe: CardCount,
}

pub fn gather_hand_count_states<F, T: Copy + Default>(
    initial_hand: &CardCount,
    initial_shoe: &CardCount,
    charlie_number: u8,
    mut feature_fn: F,
    record: &SingleStateArray<T>,
) -> Vec<Vec<HandShoePair>>
where
    F: FnMut(&CardCount) -> usize,
{
    let mut ret = Vec::new();
    let mut hand = initial_hand.clone();
    let mut shoe = initial_shoe.clone();
    gather_hand_count_states_aux(
        &charlie_number,
        &mut feature_fn,
        record,
        &mut hand,
        &mut shoe,
        1,
        &mut ret,
    );
    ret
}

fn gather_hand_count_states_aux<F, T: Copy + Default>(
    charlie_number: &u8,
    feature_fn: &mut F,
    record: &SingleStateArray<T>,
    current_card_count: &mut CardCount,
    current_shoe_count: &mut CardCount,
    loop_start_card: u8,

    result: &mut Vec<Vec<HandShoePair>>,
) where
    F: FnMut(&CardCount) -> usize,
{
    if record.contains_state(current_card_count) {
        return;
    }
    let feature = feature_fn(current_card_count);
    while result.len() <= feature {
        result.push(vec![]);
    }
    result[feature].push(HandShoePair {
        hand: current_card_count.clone(),
        shoe: current_shoe_count.clone(),
    });

    if current_card_count.get_sum() >= 21
        || current_card_count.get_total() == *charlie_number as u16
    {
        return;
    }

    for i in loop_start_card..=10 {
        if current_shoe_count[i] == 0 {
            continue;
        }
        current_shoe_count.remove_card(i);
        current_card_count.add_card(i);
        gather_hand_count_states_aux(
            charlie_number,
            feature_fn,
            record,
            current_card_count,
            current_shoe_count,
            i,
            result,
        );
        current_card_count.remove_card(i);
        current_shoe_count.add_card(i);
    }
}

pub fn gather_dealer_count_states<F>(
    dealer_hit_on_soft17: bool,
    mut feature_fn: F,
) -> Vec<Vec<CardCount>>
where
    F: FnMut(&CardCount) -> usize,
{
    let mut ret = Vec::new();
    let mut card_count = CardCount::with_number_of_decks(0);
    let mut is_visited = SingleStateArray::new();
    gather_dealer_count_states_aux(
        &dealer_hit_on_soft17,
        &mut feature_fn,
        &mut card_count,
        &mut is_visited,
        &mut ret,
    );
    ret
}

fn gather_dealer_count_states_aux<F>(
    dealer_hit_on_soft17: &bool,
    feature_fn: &mut F,
    current_card_count: &mut CardCount,
    is_visited: &mut SingleStateArray<()>,

    result: &mut Vec<Vec<CardCount>>,
) where
    F: FnMut(&CardCount) -> usize,
{
    if is_visited.contains_state(current_card_count) {
        return;
    }
    is_visited[current_card_count] = ();

    let feature = feature_fn(current_card_count);
    while result.len() <= feature {
        result.push(vec![]);
    }
    result[feature].push(current_card_count.clone());

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
    fn get_number_of_hand_states_double_group() {
        let charlie_number: u8 = 6;
        let f = |_: &CardCount| 0;
        let initial_hand = CardCount::with_number_of_decks(0);
        let initial_shoe = CardCount::with_number_of_decks(8);
        let gathered_states = gather_hand_count_states(
            &initial_hand,
            &initial_shoe,
            charlie_number,
            f,
            &SingleStateArray::<()>::new(),
        );

        let mut mark: SingleStateArray<u32> = Default::default();
        for state1 in &gathered_states[0] {
            let mut sum_state = state1.hand.clone();
            for state2 in &gathered_states[0] {
                sum_state.fast_add_assign(&state2.hand);
                mark[&sum_state] += 1;
                sum_state.fast_sub_assign(&state2.hand);
            }
        }

        println!("Number of states with double hands: {}", mark.len());
    }

    #[test]
    #[ignore]
    fn get_number_of_hand_states_single_group() {
        let charlie_number: u8 = 6;
        let f = |card_count: &CardCount| card_count.get_sum() as usize;
        let initial_hand = CardCount::with_number_of_decks(0);
        let initial_shoe = CardCount::with_number_of_decks(8);
        let gathered_states = gather_hand_count_states(
            &initial_hand,
            &initial_shoe,
            charlie_number,
            f,
            &SingleStateArray::<()>::new(),
        );
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
