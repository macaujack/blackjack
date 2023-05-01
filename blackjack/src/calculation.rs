use super::{Decision, PeekPolicy, Rule};
use crate::{CardCount, InitialSituation, StateArray};
use std::{cmp::Ordering, ops};

mod calculation_states;

#[derive(Clone, Copy, Debug)]
pub struct MaxExpectation {
    pub hit: f64,
    pub stand: f64,
}

impl Default for MaxExpectation {
    fn default() -> Self {
        MaxExpectation {
            hit: -f64::INFINITY,
            stand: -f64::INFINITY,
        }
    }
}

pub fn get_max_expectation(
    solution: &StateArray<MaxExpectation>,
    state: &CardCount,
    rule: &Rule,
) -> (f64, Decision) {
    if state.bust() {
        return (-1.0, Decision::Stand);
    }
    if state.get_total() >= rule.charlie_number as u16 {
        return (1.0, Decision::Stand);
    }

    let (mut max_ex, mut max_decision) = {
        if rule.allow_late_surrender {
            (-0.5, Decision::Surrender)
        } else {
            (-f64::INFINITY, Decision::PlaceHolder)
        }
    };

    let ex = solution[state];
    if max_ex < ex.stand {
        max_ex = ex.stand;
        max_decision = Decision::Stand;
    }
    if max_ex < ex.hit {
        max_ex = ex.hit;
        max_decision = Decision::Hit;
    }

    (max_ex, max_decision)
}

#[derive(Default)]
pub struct SolutionForInitialSituation {
    /// Note that this doesn't take the following cases into consideration:
    /// 1. Split pairs
    /// 2. Buy insurance
    pub general_solution: StateArray<MaxExpectation>,
    pub double_expectation: f64,
    pub split_expectation: f64,
}

pub fn calculate_solution(
    rule: &Rule,
    initial_situation: &InitialSituation,
) -> SolutionForInitialSituation {
    let mut general_solution: StateArray<MaxExpectation> = StateArray::new();
    let mut initial_hand = CardCount::new(&[0; 10]);
    initial_hand.add_card(initial_situation.hand_cards.0);
    initial_hand.add_card(initial_situation.hand_cards.1);
    let mut shoe = initial_situation.shoe;
    memoization_find_solution(
        rule,
        &initial_situation.dealer_up_card,
        &mut shoe,
        &mut initial_hand,
        &mut general_solution,
    );

    let mut double_expectation = 0.0;
    let current_hand = &mut initial_hand;
    for i in 1..=10 {
        current_hand.add_card(i);
        let p = shoe.get_proportion(i);
        double_expectation += p * general_solution[current_hand].stand;
        current_hand.remove_card(i);
    }
    double_expectation *= 2.0;

    // TODO: Calculate the expectation when able to split.
    SolutionForInitialSituation {
        general_solution,
        double_expectation,
        split_expectation: -6666.0,
    }
}

fn memoization_find_solution(
    // Input parameters
    rule: &Rule,
    dealer_up_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount,
    current_hand: &mut CardCount,

    // Output parameters
    solution: &mut StateArray<MaxExpectation>,
) {
    if solution.contains_state(current_hand) {
        return;
    }

    let current_sum = current_hand.get_sum();
    // Obvious case 1: Bust
    if current_sum > 21 {
        solution[current_hand] = MaxExpectation {
            stand: -1.0,
            ..Default::default()
        };
        return;
    }

    // Obvious case 2: Charlie number reached.
    if current_hand.get_total() >= rule.charlie_number as u16 {
        solution[current_hand] = MaxExpectation {
            stand: 1.0,
            ..Default::default()
        }
    }

    let stand_odds = calculate_stand_odds(rule, current_hand, dealer_up_card, current_shoe);
    // Obvious case 3: Current hard hand sum is 21. Stand!
    if current_sum == 21 {
        // Stand (obvious)
        solution[current_hand] = MaxExpectation {
            stand: stand_odds.win - stand_odds.lose,
            ..Default::default()
        };
        return;
    }
    // Obvious case 4: Current soft hand sum is 21. Stand!
    if current_hand.is_soft() && current_sum == 11 {
        let stand = {
            if current_hand.get_total() == 2 {
                // Blackjack! Congrats!
                stand_odds.win * rule.payout_blackjack - stand_odds.lose
            } else {
                stand_odds.win - stand_odds.lose
            }
        };
        solution[current_hand] = MaxExpectation {
            stand,
            ..Default::default()
        };
        return;
    }

    solution[current_hand] = MaxExpectation {
        hit: 0.0,
        ..Default::default()
    };

    for i in 1..=10 {
        if current_shoe[i] == 0 {
            continue;
        }

        current_shoe.remove_card(i);
        current_hand.add_card(i);

        memoization_find_solution(rule, dealer_up_card, current_shoe, current_hand, solution);

        let (ex_max, _): (f64, _) = get_max_expectation(solution, current_hand, rule);

        current_hand.remove_card(i);
        current_shoe.add_card(i);

        let p = current_shoe.get_proportion(i);
        solution[current_hand].hit += p * ex_max;
    }

    solution[current_hand].stand = {
        // Optimization here. No need to calculate stand odds when player's hands is <= 11 and total number of cards != 3, because
        // in this case, player should obviously hit.
        // When total number of cards is 3, we still need to calculate stand odds, because the stand expectation is used to
        // calculate double expectation.
        if current_hand.get_actual_sum() <= 11 && current_hand.get_total() != 3 {
            -f64::INFINITY
        } else {
            let stand_odds = calculate_stand_odds(rule, current_hand, dealer_up_card, current_shoe);
            stand_odds.win - stand_odds.lose
        }
    };
}

#[derive(Clone, Copy, Default, Debug)]
struct WinLoseCasesOdds {
    win: f64,
    push: f64,
    lose: f64,
}

impl ops::AddAssign<&WinLoseCasesOdds> for WinLoseCasesOdds {
    fn add_assign(&mut self, rhs: &WinLoseCasesOdds) {
        self.win += rhs.win;
        self.push += rhs.push;
        self.lose += rhs.lose;
    }
}

impl ops::Mul<f64> for WinLoseCasesOdds {
    type Output = WinLoseCasesOdds;
    fn mul(self, rhs: f64) -> Self::Output {
        WinLoseCasesOdds {
            win: self.win * rhs,
            push: self.push * rhs,
            lose: self.lose * rhs,
        }
    }
}

fn calculate_stand_odds(
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
                10 => shoe.get_proportion(1),
                _ => 0.0,
            },
            PeekPolicy::NoPeek => match *dealer_up_card {
                1 => shoe.get_proportion(10),
                10 => shoe.get_proportion(1),
                _ => 0.0,
            },
        };
        return WinLoseCasesOdds {
            win: 1.0 - p_dealer_also_natural,
            push: p_dealer_also_natural,
            lose: 0.0,
        };
    }

    let mut odds = StateArray::new();

    memoization_find_win_lose_odds(
        rule,
        &player_sum,
        dealer_up_card,
        &shoe,
        &mut dealer_extra_hand,
        &mut odds,
    );

    odds[&dealer_extra_hand]
}

/// Note that the callers of this function must ensure that if player_sum is 21, it must NOT be
/// a natural Blackjack. Player natural Blackjack should be handled separately as a special
/// case before recursively calling this function.
fn memoization_find_win_lose_odds(
    // Input parameters
    rule: &Rule,
    player_sum: &u16,
    dealer_up_card: &u8,
    original_shoe: &CardCount, // Original cards in the shoe just before dealer's hole card is revealed

    // Parameters to maintain current state
    dealer_extra_hand: &mut CardCount, // Dealer's hand except for the up card
    odds: &mut StateArray<WinLoseCasesOdds>,
) {
    if odds.contains_state(dealer_extra_hand) {
        return;
    }

    // Case 1: Dealer must stand.
    let dealer_sum = dealer_extra_hand.get_sum() + (*dealer_up_card as u16);
    let is_soft = dealer_extra_hand.is_soft() || *dealer_up_card == 1;
    if dealer_sum > 21 {
        odds[dealer_extra_hand] = WinLoseCasesOdds {
            win: 1.0,
            ..Default::default()
        };
        return;
    }
    if dealer_sum >= 17 {
        add_to_win_lose_cases_count(*player_sum, dealer_sum, &mut odds[dealer_extra_hand], 1.0);
        return;
    }
    if is_soft {
        // Dealer gets natural Blackjack!! OMG!!
        // Note that if the peek policy is UpAceOrTen, dealer will peek the hole card when the up card is Ace or 10,
        // which immediately ends the game if she gets a natural Blackjack. This in turn makes the following 'if'
        // impossible to run.
        if dealer_sum + 10 == 21 && dealer_extra_hand.get_total() == 1 {
            odds[dealer_extra_hand] = WinLoseCasesOdds {
                lose: 1.0,
                ..Default::default()
            };
            return;
        }

        let lower_bound = {
            if rule.dealer_hit_on_soft17 {
                18
            } else {
                17
            }
        };
        if dealer_sum + 10 >= lower_bound && dealer_sum + 10 <= 21 {
            add_to_win_lose_cases_count(
                *player_sum,
                dealer_sum + 10,
                &mut odds[dealer_extra_hand],
                1.0,
            );
            return;
        }
    }

    // Case 2: Dealer must hit.
    let (next_card_min, next_card_max, current_valid_shoe_total) = {
        if dealer_extra_hand.get_total() != 0 {
            (
                1,
                10,
                original_shoe.get_total() - dealer_extra_hand.get_total(),
            )
        } else {
            // Yes this is an ugly piece of code. If Rust supports 'fallthrough' in the pattern matching,
            // the code can be much cleaner.
            match rule.peek_policy {
                PeekPolicy::UpAceOrTen => match *dealer_up_card {
                    1 => (1, 9, original_shoe.get_total() - original_shoe[10]),
                    10 => (2, 10, original_shoe.get_total() - original_shoe[1]),
                    _ => (1, 10, original_shoe.get_total()),
                },
                PeekPolicy::UpAce => match *dealer_up_card {
                    1 => (1, 9, original_shoe.get_total() - original_shoe[10]),
                    _ => (1, 10, original_shoe.get_total()),
                },
                PeekPolicy::NoPeek => (
                    1,
                    10,
                    original_shoe.get_total() - dealer_extra_hand.get_total(),
                ),
            }
        }
    };
    let current_valid_shoe_total = current_valid_shoe_total as f64;

    for card in next_card_min..=next_card_max {
        if dealer_extra_hand[card] == original_shoe[card] {
            continue;
        }

        dealer_extra_hand.add_card(card);
        memoization_find_win_lose_odds(
            rule,
            player_sum,
            dealer_up_card,
            original_shoe,
            dealer_extra_hand,
            odds,
        );
        let next_state_odds = odds[dealer_extra_hand];
        dealer_extra_hand.remove_card(card);

        odds[dealer_extra_hand] += &(next_state_odds
            * (((original_shoe[card] - dealer_extra_hand[card]) as f64)
                / current_valid_shoe_total));
    }
}

fn add_to_win_lose_cases_count(
    player_sum: u16,
    dealer_sum: u16,
    count: &mut WinLoseCasesOdds,
    delta: f64,
) {
    match player_sum.cmp(&dealer_sum) {
        Ordering::Less => count.lose += delta,
        Ordering::Equal => count.push += delta,
        Ordering::Greater => count.win += delta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_typical_rule() -> Rule {
        Rule {
            number_of_decks: 8,
            cut_card_proportion: 0.5,
            split_all_limits: 1,
            split_ace_limits: 1,
            double_policy: crate::DoublePolicy::AnyTwo,
            dealer_hit_on_soft17: false,
            allow_das: false,
            allow_late_surrender: false,
            peek_policy: crate::PeekPolicy::UpAce,
            charlie_number: 6,

            payout_blackjack: 1.5,
            payout_insurance: 0.0,
        }
    }

    #[test]
    #[ignore]
    fn test_find_win_lose_cases_count() {
        let rule = get_typical_rule();
        let original_shoe = CardCount::new(&[0, 0, 1, 0, 0, 0, 1, 0, 0, 1]);
        let mut dealer_extra_hand = CardCount::new(&[0; 10]);
        let mut odds = StateArray::new();
        memoization_find_win_lose_odds(
            &rule,
            &18,
            &1,
            &original_shoe,
            &mut dealer_extra_hand,
            &mut odds,
        );

        let od = odds[&CardCount::new(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0])];
        println!("{:#?}", od);
        println!("{:#?}", od.win + od.push + od.lose);
    }

    #[test]
    #[ignore]
    fn test_decision() {
        let rule = get_typical_rule();

        let mut counts = [4 * (rule.number_of_decks as u16); 10];
        counts[9] = 16 * (rule.number_of_decks as u16);
        let mut shoe = CardCount::new(&counts);
        let hand_cards = (9, 2);
        let dealer_up_card = 1;
        shoe.remove_card(hand_cards.0);
        shoe.remove_card(hand_cards.1);
        shoe.remove_card(dealer_up_card);

        let initial_situation = InitialSituation {
            shoe,
            hand_cards,
            dealer_up_card,
        };

        let sol = calculate_solution(&rule, &initial_situation);
        let mut initial_hand = CardCount::new(&[0; 10]);
        initial_hand.add_card(hand_cards.0);
        initial_hand.add_card(hand_cards.1);
        println!("{:#?}", sol.general_solution[&initial_hand]);
    }

    #[test]
    #[ignore]
    fn print_basic_strategy() {
        let rule = get_typical_rule();

        let mut counts = [4 * (rule.number_of_decks as u16); 10];
        counts[9] = 16 * (rule.number_of_decks as u16);
        let counts = counts;

        println!("Hard:");
        for my_hand_total in 5..=18 {
            for dealer_up_card in [2, 3, 4, 5, 6, 7, 8, 9, 10, 1] {
                let mut shoe = CardCount::new(&counts);
                let hand_cards = {
                    if my_hand_total - 2 <= 10 {
                        (2, my_hand_total - 2)
                    } else {
                        (10, my_hand_total - 10)
                    }
                };
                shoe.remove_card(hand_cards.0);
                shoe.remove_card(hand_cards.1);
                shoe.remove_card(dealer_up_card);

                let initial_situation = InitialSituation {
                    shoe,
                    hand_cards,
                    dealer_up_card,
                };

                let sol = calculate_solution(&rule, &initial_situation);
                let mut initial_hand = CardCount::new(&[0; 10]);
                initial_hand.add_card(hand_cards.0);
                initial_hand.add_card(hand_cards.1);
                let (max_ex, mut decision) =
                    get_max_expectation(&sol.general_solution, &initial_hand, &rule);
                if max_ex < sol.double_expectation {
                    decision = Decision::Double;
                }
                print!("{} ", decision_to_char(decision));
            }
            println!();
        }

        println!();
        println!("Soft:");

        for another_card in 2..=9 {
            for dealer_up_card in [2, 3, 4, 5, 6, 7, 8, 9, 10, 1] {
                let mut shoe = CardCount::new(&counts);
                let hand_cards = (1, another_card);
                shoe.remove_card(hand_cards.0);
                shoe.remove_card(hand_cards.1);
                shoe.remove_card(dealer_up_card);

                let initial_situation = InitialSituation {
                    shoe,
                    hand_cards,
                    dealer_up_card,
                };

                let sol = calculate_solution(&rule, &initial_situation);
                let mut initial_hand = CardCount::new(&[0; 10]);
                initial_hand.add_card(hand_cards.0);
                initial_hand.add_card(hand_cards.1);
                let (max_ex, mut decision) =
                    get_max_expectation(&sol.general_solution, &initial_hand, &rule);
                if max_ex < sol.double_expectation {
                    decision = Decision::Double;
                }
                print!("{} ", decision_to_char(decision));
            }
            println!();
        }
    }

    fn decision_to_char(decision: Decision) -> char {
        match decision {
            Decision::Hit => 'H',
            Decision::Stand => 'S',
            Decision::Double => 'D',
            Decision::Surrender => 'R',
            _ => panic!("wtf"),
        }
    }
}
