use super::calculation_states;
use super::calculation_states::HandShoePair;
use super::stand_odds::{memoization_dealer_get_cards, DealerHandHandler};
use super::{
    get_card_probability, get_max_expectation_of_stand_hit_surrender, ExpectationStandHit,
};
use crate::{CardCount, PeekPolicy, Rule, SingleStateArray};
use std::cmp::Ordering;

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
        &impossible_dealer_hole_card,
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
                let p = get_card_probability(&pair.shoe, impossible_dealer_hole_card, next_card);
                if p == 0.0 {
                    continue;
                }
                pair.hand.add_card(next_card);
                let (ex_max, _) =
                    get_max_expectation_of_stand_hit_surrender(ex_stand_hit, &pair.hand, rule);
                pair.hand.remove_card(next_card);
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
        let p = get_card_probability(current_shoe, *impossible_dealer_hole_card, i);
        if p == 0.0 {
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

#[derive(Clone, Default, Debug)]
struct WinLoseCasesOdds {
    win: f64,
    lose: f64,
}

impl DealerHandHandler for WinLoseCasesOdds {
    fn end_with_dealer_bust(&mut self) {
        self.win = 1.0;
    }

    fn end_with_dealer_normal(&mut self, dealer_actual_sum: u16, player_actual_sum: u16) {
        match player_actual_sum.cmp(&dealer_actual_sum) {
            Ordering::Less => self.lose = 1.0,
            Ordering::Equal => {}
            Ordering::Greater => self.win = 1.0,
        }
    }

    fn end_with_dealer_natural(&mut self) {
        self.lose = 1.0;
    }

    fn add_assign_with_p(&mut self, rhs: &Self, p: f64) {
        self.win += rhs.win * p;
        self.lose += rhs.lose * p;
    }
}

/// Note that this function doesn't consider the situation where player's hand
/// reach Charlie number. This case should be handled separately before calling
/// this function.
fn calculate_stand_odds_single_hand(
    rule: &Rule,
    player_hand: &CardCount,
    dealer_up_card: &u8,
    shoe: &CardCount,
) -> WinLoseCasesOdds {
    let mut dealer_extra_hand = CardCount::new(&[0; 10]);
    let player_sum = player_hand.get_actual_sum();

    // Special case: Player hand is natural Blackjack
    if player_hand.is_natural() {
        let p_dealer_also_natural = match rule.peek_policy {
            PeekPolicy::UpAceOrTen => 0.0,
            PeekPolicy::UpAce => match *dealer_up_card {
                10 => get_card_probability(shoe, 0, 1),
                _ => 0.0,
            },
            PeekPolicy::NoPeek => match *dealer_up_card {
                1 => get_card_probability(shoe, 0, 10),
                10 => get_card_probability(shoe, 0, 1),
                _ => 0.0,
            },
        };
        return WinLoseCasesOdds {
            win: 1.0 - p_dealer_also_natural,
            lose: 0.0,
        };
    }

    let mut odds = SingleStateArray::new();
    let (next_card_min, next_card_max) = match rule.peek_policy {
        PeekPolicy::UpAceOrTen => match *dealer_up_card {
            1 => (1, 9),
            10 => (2, 10),
            _ => (1, 10),
        },
        PeekPolicy::UpAce => match *dealer_up_card {
            1 => (1, 9),
            _ => (1, 10),
        },
        PeekPolicy::NoPeek => (1, 10),
    };

    match (next_card_min, next_card_max) {
        (1, 10) => memoization_dealer_get_cards::<WinLoseCasesOdds, 1, 10>(
            rule,
            &player_sum,
            dealer_up_card,
            &shoe,
            &mut dealer_extra_hand,
            &mut odds,
        ),
        (1, 9) => memoization_dealer_get_cards::<WinLoseCasesOdds, 1, 9>(
            rule,
            &player_sum,
            dealer_up_card,
            &shoe,
            &mut dealer_extra_hand,
            &mut odds,
        ),
        (2, 10) => memoization_dealer_get_cards::<WinLoseCasesOdds, 2, 10>(
            rule,
            &player_sum,
            dealer_up_card,
            &shoe,
            &mut dealer_extra_hand,
            &mut odds,
        ),
        _ => panic!("Impossible to reach"),
    }

    odds[&dealer_extra_hand].clone()
}

#[cfg(test)]
mod tests {
    use super::super::tests::get_typical_rule;
    use super::*;

    #[test]
    #[ignore]
    fn test_find_win_lose_cases_count() {
        let rule = get_typical_rule();
        let original_shoe = CardCount::new(&[0, 0, 1, 0, 0, 0, 1, 0, 0, 1]);
        let mut dealer_extra_hand = CardCount::new(&[0; 10]);
        let mut odds = SingleStateArray::new();
        memoization_dealer_get_cards::<WinLoseCasesOdds, 1, 9>(
            &rule,
            &18,
            &1,
            &original_shoe,
            &mut dealer_extra_hand,
            &mut odds,
        );

        let od = &odds[&CardCount::new(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0])];
        println!("{:#?}", od);
    }
}
