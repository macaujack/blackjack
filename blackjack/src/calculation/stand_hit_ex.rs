use super::calculation_states;
use super::calculation_states::HandShoePair;
use super::stand_odds::calculate_stand_odds_single_hand;
use super::{
    get_card_probability, get_max_expectation_of_stand_hit_surrender, ExpectationStandHit,
};
use crate::{CardCount, Rule, SingleStateArray};

pub fn multithreading_calculate_stand_hit_expectation(
    // Input parameters
    number_of_threads: usize,
    rule: &Rule,
    dealer_up_card: u8,
    impossible_dealer_hole_card: u8,

    // Parameters to maintain current state
    initial_shoe: &CardCount,
    initial_hand: &CardCount,

    // Output parameters
    ex_stand_hit: &mut SingleStateArray<ExpectationStandHit>,
) {
    let feature_fn = |c: &'_ CardCount| c.get_total() as usize;
    let mut valid_pairs = calculation_states::gather_hand_count_states(
        initial_hand,
        initial_shoe,
        rule.charlie_number,
        feature_fn,
        ex_stand_hit,
    );
    let mut dispatched_hands: Vec<Vec<HandShoePair>> = Vec::with_capacity(number_of_threads);
    for _ in 0..number_of_threads {
        dispatched_hands.push(Vec::new());
    }
    let mut state_count = 0;
    for pairs in &valid_pairs {
        for pair in pairs {
            // Obvious case 1: Bust
            if pair.hand.bust() {
                ex_stand_hit[&pair.hand] = ExpectationStandHit {
                    stand: -1.0,
                    ..Default::default()
                };
                continue;
            }

            // Obvious case 2: Charlie number reached.
            if pair.hand.get_total() == rule.charlie_number as u16 {
                ex_stand_hit[&pair.hand] = ExpectationStandHit {
                    stand: 1.0,
                    ..Default::default()
                };
                continue;
            }

            if pair.hand.get_actual_sum() <= 11 && pair.hand.get_total() != 3 {
                ex_stand_hit[&pair.hand] = ExpectationStandHit {
                    stand: -f64::INFINITY,
                    hit: 0.0,
                };
                continue;
            }
            ex_stand_hit[&pair.hand] = ExpectationStandHit {
                stand: 0.0,
                hit: 0.0,
            };

            // Obvious case 3: Current actual sum is 21. Stand!
            if pair.hand.get_actual_sum() == 21 {
                ex_stand_hit[&pair.hand] = ExpectationStandHit {
                    stand: 0.0,
                    ..Default::default()
                };
                // Don't continue here, because we want to calculate the expectation
                // of Stand.
            }
            dispatched_hands[state_count % number_of_threads].push(pair.clone());
            state_count += 1;
        }
    }

    // Calculate expectation of Stand.
    let mut threads = Vec::with_capacity(number_of_threads - 1);
    let raw_ex_stand_hit = ex_stand_hit as *mut SingleStateArray<ExpectationStandHit> as usize;
    for _ in 1..number_of_threads {
        let pairs_for_thread = dispatched_hands.pop().unwrap();
        let rule = *rule;
        let thread = std::thread::spawn(move || {
            for pair in &pairs_for_thread {
                let stand_odds = calculate_stand_odds_single_hand(
                    &rule,
                    &pair.hand,
                    &dealer_up_card,
                    &pair.shoe,
                );
                unsafe {
                    // This is OK, since the threads are not modifying the same memory.
                    let ex_stand_hit =
                        &mut *(raw_ex_stand_hit as *mut SingleStateArray<ExpectationStandHit>);
                    ex_stand_hit[&pair.hand].stand = {
                        if pair.hand.is_natural() {
                            stand_odds.win * rule.payout_blackjack - stand_odds.lose
                        } else {
                            stand_odds.win - stand_odds.lose
                        }
                    };
                }
            }
        });
        threads.push(thread);
    }
    for pair in dispatched_hands.first().unwrap() {
        let stand_odds =
            calculate_stand_odds_single_hand(&rule, &pair.hand, &dealer_up_card, &pair.shoe);
        ex_stand_hit[&pair.hand].stand = {
            if pair.hand.is_natural() {
                stand_odds.win * rule.payout_blackjack - stand_odds.lose
            } else {
                stand_odds.win - stand_odds.lose
            }
        };
    }
    for thread in threads {
        let _ = thread.join();
    }

    // Calculate expectation of Hit.
    for pairs in valid_pairs.iter_mut().rev() {
        for pair in pairs {
            if ex_stand_hit[&pair.hand].hit != 0.0 {
                continue;
            }

            for next_card in 1..=10 {
                if pair.shoe[next_card] == 0 {
                    continue;
                }
                pair.hand.add_card(next_card);
                let (ex_max, _) =
                    get_max_expectation_of_stand_hit_surrender(ex_stand_hit, &pair.hand, rule);
                pair.hand.remove_card(next_card);
                let p = get_card_probability(&pair.shoe, impossible_dealer_hole_card, next_card);
                ex_stand_hit[&pair.hand].hit += p * ex_max;
            }
        }
    }
}

pub fn memoization_calculate_stand_hit_expectation(
    // Input parameters
    rule: &Rule,
    dealer_up_card: &u8,
    impossible_dealer_hole_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount,
    current_hand: &mut CardCount,

    // Output parameters
    ex_stand_hit: &mut SingleStateArray<ExpectationStandHit>,
) {
    if ex_stand_hit.contains_state(current_hand) {
        return;
    }

    // Obvious case 1: Bust
    if current_hand.bust() {
        ex_stand_hit[current_hand] = ExpectationStandHit {
            stand: -1.0,
            ..Default::default()
        };
        return;
    }

    // Obvious case 2: Charlie number reached.
    if current_hand.get_total() == rule.charlie_number as u16 {
        ex_stand_hit[current_hand] = ExpectationStandHit {
            stand: 1.0,
            ..Default::default()
        };
        return;
    }

    // Obvious case 3: Current actual sum is 21. Stand!
    if current_hand.get_actual_sum() == 21 {
        let stand_odds =
            calculate_stand_odds_single_hand(rule, current_hand, dealer_up_card, current_shoe);

        let stand = {
            if current_hand.is_natural() {
                stand_odds.win * rule.payout_blackjack - stand_odds.lose
            } else {
                stand_odds.win - stand_odds.lose
            }
        };
        ex_stand_hit[current_hand] = ExpectationStandHit {
            stand,
            ..Default::default()
        };
        return;
    }

    // End of obvious cases. Calculate expectation of Hit using theory of total expectation.
    ex_stand_hit[current_hand] = ExpectationStandHit {
        hit: 0.0,
        ..Default::default()
    };

    for i in 1..=10 {
        if current_shoe[i] == 0 {
            continue;
        }

        current_shoe.remove_card(i);
        current_hand.add_card(i);

        memoization_calculate_stand_hit_expectation(
            rule,
            dealer_up_card,
            impossible_dealer_hole_card,
            current_shoe,
            current_hand,
            ex_stand_hit,
        );

        let (ex_max, _) =
            get_max_expectation_of_stand_hit_surrender(ex_stand_hit, current_hand, rule);

        current_hand.remove_card(i);
        current_shoe.add_card(i);

        let p = get_card_probability(current_shoe, *impossible_dealer_hole_card, i);
        ex_stand_hit[current_hand].hit += p * ex_max;
    }

    // Calculate expectation of Stand.
    ex_stand_hit[current_hand].stand = {
        // Optimization here. No need to calculate stand odds when player's hands is <= 11 and total number of cards != 3, because
        // in this case, player should obviously hit.
        // When total number of cards is 3, we still need to calculate stand odds, because the stand expectation is used to
        // calculate double expectation.
        if current_hand.get_actual_sum() <= 11 && current_hand.get_total() != 3 {
            -f64::INFINITY
        } else {
            let stand_odds =
                calculate_stand_odds_single_hand(rule, current_hand, dealer_up_card, current_shoe);
            stand_odds.win - stand_odds.lose
        }
    };
}
