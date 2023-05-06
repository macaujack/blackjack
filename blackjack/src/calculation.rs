use self::calculation_states::HandShoePair;

use super::{Decision, PeekPolicy, Rule};
use crate::{CardCount, InitialSituation, StateArray};
use std::{cmp::Ordering, ops};

mod calculation_states;

#[derive(Clone, Copy, Debug)]
pub struct Expectation {
    pub hit: f64,
    pub stand: f64,
}

impl Default for Expectation {
    fn default() -> Self {
        Expectation {
            hit: -f64::INFINITY,
            stand: -f64::INFINITY,
        }
    }
}

pub fn get_max_expectation(
    solution: &StateArray<Expectation>,
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

#[derive(Debug, Default)]
pub struct SolutionForInitialSituation {
    pub ex_stand_hit: StateArray<Expectation>,
    pub ex_double: f64,
    pub ex_split: f64,

    /// Represents the expectation of the side bet "Buy Insurance". There is no relation between this side
    /// bet and the main game. If this expectation is positive, players should buy insurance.
    /// Note that this expectation is based on its own bet, not the main bet.
    pub ex_extra_insurance: f64,

    /// Represents the final answer to "How much (mathematical expectation) can I get under this initial
    /// situation?". In most cases when dealer doesn't peek, this expectation equals to the maximum
    /// expectation among expectations of all decisions. In other cases when dealer peeks, this expectation
    /// will not only involve the maximum expectation of decisions (this expectation is under the situation
    /// where the game continues after dealer peeks), but also involve the expectation under the situation
    /// where the game ends because dealer peeks and gets natural blackjack.
    pub ex_summary: f64,
}

#[derive(Debug, Default, Clone, Copy)]
struct ExsOtherDecisions {
    ex_double: f64,
    ex_split: f64,

    ex_extra_insurance: f64,

    ex_summary: f64,
}

const fn get_prefix_sum() -> [usize; 10] {
    let mut ret = [0; 10];
    let mut i = 1;
    while i < ret.len() {
        ret[i] = ret[i - 1] + i;
        i += 1;
    }

    ret
}

static PREFIX_SUM: [usize; 10] = get_prefix_sum();

#[derive(Debug)]
pub struct SolutionForBettingPhase {
    exs_stand_hit: [StateArray<Expectation>; 10],
    exs_other_decisions: [[ExsOtherDecisions; 55]; 10],
    ex_total_summary: f64,
}

impl Default for SolutionForBettingPhase {
    fn default() -> Self {
        let exs_other_decisions = [[Default::default(); 55]; 10];
        SolutionForBettingPhase {
            exs_stand_hit: Default::default(),
            exs_other_decisions,
            ex_total_summary: Default::default(),
        }
    }
}

impl SolutionForBettingPhase {
    pub fn into_solution_for_initial_situation(
        mut self,
        hand: (u8, u8),
        dealer_up_card: u8,
    ) -> SolutionForInitialSituation {
        let mut sol = self.get_solution_for_initial_situation_aux(hand, dealer_up_card);
        sol.ex_stand_hit = core::mem::take(&mut self.exs_stand_hit[(dealer_up_card - 1) as usize]);
        sol
    }

    pub fn get_solution_for_initial_situation(
        &self,
        hand: (u8, u8),
        dealer_up_card: u8,
    ) -> SolutionForInitialSituation {
        let mut sol = self.get_solution_for_initial_situation_aux(hand, dealer_up_card);
        sol.ex_stand_hit = self.exs_stand_hit[(dealer_up_card - 1) as usize].clone();
        sol
    }

    pub fn get_total_expectation(&self) -> f64 {
        self.ex_total_summary
    }

    fn get_solution_for_initial_situation_aux(
        &self,
        mut hand: (u8, u8),
        dealer_up_card: u8,
    ) -> SolutionForInitialSituation {
        if hand.0 < hand.1 {
            (hand.0, hand.1) = (hand.1, hand.0);
        }
        let a = (hand.0 - 1) as usize;
        let b = (hand.1 - 1) as usize;
        let d = (dealer_up_card - 1) as usize;
        let other = &self.exs_other_decisions[d][PREFIX_SUM[a] + b];
        SolutionForInitialSituation {
            ex_stand_hit: Default::default(),
            ex_double: other.ex_double,
            ex_split: other.ex_split,
            ex_extra_insurance: other.ex_extra_insurance,
            ex_summary: other.ex_summary,
        }
    }
}

fn get_card_probability(shoe: &CardCount, impossible_dealer_hole_card: u8, target_card: u8) -> f64 {
    let total = shoe.get_total() as f64;
    let target_number = shoe[target_card] as f64;
    if impossible_dealer_hole_card == 0 {
        return target_number / total;
    }

    let p_hole_card_is_target_card = {
        if impossible_dealer_hole_card == target_card {
            0.0
        } else {
            target_number / (shoe.get_total() - shoe[impossible_dealer_hole_card]) as f64
        }
    };
    let shoe_total_minus_one = (shoe.get_total() - 1) as f64;
    let p1 = p_hole_card_is_target_card * (shoe[target_card] - 1) as f64 / shoe_total_minus_one;
    let p2 = (1.0 - p_hole_card_is_target_card) * target_number / shoe_total_minus_one;
    p1 + p2
}

fn get_impossible_dealer_hole_card(rule: &Rule, dealer_up_card: u8) -> u8 {
    match rule.peek_policy {
        PeekPolicy::UpAceOrTen => match dealer_up_card {
            1 => 10,
            10 => 1,
            _ => 0,
        },
        PeekPolicy::UpAce => match dealer_up_card {
            1 => 10,
            _ => 0,
        },
        PeekPolicy::NoPeek => 0,
    }
}

fn get_number_of_threads(number_of_threads: usize) -> usize {
    if number_of_threads == 0 {
        let ret = std::thread::available_parallelism();
        match ret {
            Ok(x) => x.get(),
            Err(_) => 1,
        }
    } else {
        number_of_threads
    }
}

/// Calculates the expectation under the situation where dealer gets each card.
pub fn calculate_solution_without_initial_situation(
    number_of_threads: usize,
    rule: &Rule,
    shoe: &CardCount,
) -> SolutionForBettingPhase {
    let number_of_threads = get_number_of_threads(number_of_threads);
    let mut solution: SolutionForBettingPhase = Default::default();

    let mut initial_situation = InitialSituation::new(shoe.clone(), (1, 1), 1);
    let total_combs = rule.number_of_decks as u32 * 52;
    let total_combs = total_combs * (total_combs - 1) * (total_combs - 2);
    let total_combs = total_combs as f64;
    // Enumerate all possible combinations.
    for dealer_up_card in 1..=10 {
        let idx10 = (dealer_up_card - 1) as usize;
        initial_situation.dealer_up_card = dealer_up_card;
        let combs = initial_situation.shoe[dealer_up_card] as u32;
        initial_situation.shoe.remove_card(dealer_up_card);
        for first_hand_card in 1..=10 {
            initial_situation.hand_cards.0 = first_hand_card;
            let combs = combs * initial_situation.shoe[first_hand_card] as u32;
            initial_situation.shoe.remove_card(first_hand_card);
            for second_hand_card in 1..=first_hand_card {
                let idx55 =
                    PREFIX_SUM[(first_hand_card - 1) as usize] + (second_hand_card - 1) as usize;
                initial_situation.hand_cards.1 = second_hand_card;
                let mut combs = combs * initial_situation.shoe[second_hand_card] as u32;
                if second_hand_card != first_hand_card {
                    combs *= 2;
                }
                let combs = combs;
                initial_situation.shoe.remove_card(second_hand_card);

                // Core logic
                let p = combs as f64 / total_combs;
                let ex_other = calculate_expectations(
                    number_of_threads,
                    rule,
                    &initial_situation,
                    &mut solution.exs_stand_hit[idx10],
                );
                solution.exs_other_decisions[idx10][idx55] = ex_other;
                solution.ex_total_summary += p * ex_other.ex_summary;

                initial_situation.shoe.add_card(second_hand_card);
            }
            initial_situation.shoe.add_card(first_hand_card);
        }
        initial_situation.shoe.add_card(dealer_up_card);
    }

    solution
}

/// Note that this function hasn't considered Split yet.
pub fn calculate_solution_with_initial_situation(
    number_of_threads: usize,
    rule: &Rule,
    initial_situation: &InitialSituation,
) -> SolutionForInitialSituation {
    let number_of_threads = get_number_of_threads(number_of_threads);
    let mut ex_stand_hit = StateArray::new();

    // Calculate expectation of Stand and Hit.
    let exs_other = calculate_expectations(
        number_of_threads,
        rule,
        initial_situation,
        &mut ex_stand_hit,
    );

    // TODO: Calculate the expectation when able to split.
    SolutionForInitialSituation {
        ex_stand_hit,
        ex_double: exs_other.ex_double,
        ex_split: exs_other.ex_split,
        ex_extra_insurance: exs_other.ex_extra_insurance,
        ex_summary: exs_other.ex_summary,
    }
}

// Updates the expectations of Stand and Hit in the input parameter ex_stand_hit.
// Returns the expectations of other decisions in the return value.
// If the given number_of_threads is 0, the function will use
// std::thread::available_parallelism to get the threads.
fn calculate_expectations(
    number_of_threads: usize,
    rule: &Rule,
    initial_situation: &InitialSituation,
    ex_stand_hit: &mut StateArray<Expectation>,
) -> ExsOtherDecisions {
    let mut initial_hand = CardCount::with_number_of_decks(0);
    initial_hand.add_card(initial_situation.hand_cards.0);
    initial_hand.add_card(initial_situation.hand_cards.1);
    let mut shoe = initial_situation.shoe.clone();
    let impossible_dealer_hole_card =
        get_impossible_dealer_hole_card(rule, initial_situation.dealer_up_card);

    // Calculate expectation of Stand and hit.
    if number_of_threads <= 1 {
        memoization_calculate_stand_hit_expectation(
            rule,
            &initial_situation.dealer_up_card,
            &impossible_dealer_hole_card,
            &mut shoe,
            &mut initial_hand,
            ex_stand_hit,
        );
    } else {
        multithreading_calculate_stand_hit_expectation(
            number_of_threads,
            rule,
            initial_situation.dealer_up_card,
            impossible_dealer_hole_card,
            &shoe,
            &initial_hand,
            ex_stand_hit,
        );
    }

    // Calculate expectation of Double.
    let ex_double = {
        if initial_hand.is_natural() {
            -f64::INFINITY
        } else {
            let mut ex_double = 0.0;
            for third_card in 1..=10 {
                initial_hand.add_card(third_card);
                let p = get_card_probability(
                    &initial_situation.shoe,
                    impossible_dealer_hole_card,
                    third_card,
                );
                ex_double += p * ex_stand_hit[&initial_hand].stand;
                initial_hand.remove_card(third_card);
            }
            ex_double * 2.0
        }
    };

    // TODO: Calculate expectation of Split

    // Calculate extra expectation of side bet "Buy Insurance".
    let p_early_end = {
        if impossible_dealer_hole_card == 0 {
            0.0
        } else {
            get_card_probability(&initial_situation.shoe, 0, impossible_dealer_hole_card)
        }
    };
    let ex_extra_insurance = p_early_end * rule.payout_insurance - (1.0 - p_early_end);

    // Calculate expectation summary.
    let mut ex_early_end = {
        if initial_hand.is_natural() {
            0.0
        } else {
            -1.0
        }
    };
    if ex_extra_insurance > 0.0 {
        // Here we multiply by 0.5, because we can only spend half of main bet buying insurance.
        ex_early_end += ex_extra_insurance * 0.5;
    }
    let ex_no_early_end = {
        let (mut ex, _) = get_max_expectation(&ex_stand_hit, &initial_hand, rule);
        if ex < ex_double {
            ex = ex_double;
        }
        // TODO: Compare Split EX here.
        ex
    };
    let ex_summary = p_early_end * ex_early_end + (1.0 - p_early_end) * ex_no_early_end;

    ExsOtherDecisions {
        ex_double,
        ex_split: -f64::INFINITY,
        ex_extra_insurance,
        ex_summary,
    }
}

fn multithreading_calculate_stand_hit_expectation(
    // Input parameters
    number_of_threads: usize,
    rule: &Rule,
    dealer_up_card: u8,
    impossible_dealer_hole_card: u8,

    // Parameters to maintain current state
    initial_shoe: &CardCount,
    initial_hand: &CardCount,

    // Output parameters
    ex_stand_hit: &mut StateArray<Expectation>,
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
                ex_stand_hit[&pair.hand] = Expectation {
                    stand: -1.0,
                    ..Default::default()
                };
                continue;
            }

            // Obvious case 2: Charlie number reached.
            if pair.hand.get_total() == rule.charlie_number as u16 {
                ex_stand_hit[&pair.hand] = Expectation {
                    stand: 1.0,
                    ..Default::default()
                };
                continue;
            }

            if pair.hand.get_actual_sum() <= 11 && pair.hand.get_total() != 3 {
                ex_stand_hit[&pair.hand] = Expectation {
                    stand: -f64::INFINITY,
                    hit: 0.0,
                };
                continue;
            }
            ex_stand_hit[&pair.hand] = Expectation {
                stand: 0.0,
                hit: 0.0,
            };

            // Obvious case 3: Current actual sum is 21. Stand!
            if pair.hand.get_actual_sum() == 21 {
                ex_stand_hit[&pair.hand] = Expectation {
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
    let raw_ex_stand_hit = ex_stand_hit as *mut StateArray<Expectation> as usize;
    for _ in 1..number_of_threads {
        let pairs_for_thread = dispatched_hands.pop().unwrap();
        let rule = *rule;
        let thread = std::thread::spawn(move || {
            for pair in &pairs_for_thread {
                let stand_odds =
                    calculate_stand_odds(&rule, &pair.hand, &dealer_up_card, &pair.shoe);
                unsafe {
                    // This is OK, since the threads are not modifying the same memory.
                    let ex_stand_hit = &mut *(raw_ex_stand_hit as *mut StateArray<Expectation>);
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
        let stand_odds = calculate_stand_odds(&rule, &pair.hand, &dealer_up_card, &pair.shoe);
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
                let (ex_max, _) = get_max_expectation(ex_stand_hit, &pair.hand, rule);
                pair.hand.remove_card(next_card);
                let p = get_card_probability(&pair.shoe, impossible_dealer_hole_card, next_card);
                ex_stand_hit[&pair.hand].hit += p * ex_max;
            }
        }
    }
}

fn memoization_calculate_stand_hit_expectation(
    // Input parameters
    rule: &Rule,
    dealer_up_card: &u8,
    impossible_dealer_hole_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount,
    current_hand: &mut CardCount,

    // Output parameters
    ex_stand_hit: &mut StateArray<Expectation>,
) {
    if ex_stand_hit.contains_state(current_hand) {
        return;
    }

    // Obvious case 1: Bust
    if current_hand.bust() {
        ex_stand_hit[current_hand] = Expectation {
            stand: -1.0,
            ..Default::default()
        };
        return;
    }

    // Obvious case 2: Charlie number reached.
    if current_hand.get_total() == rule.charlie_number as u16 {
        ex_stand_hit[current_hand] = Expectation {
            stand: 1.0,
            ..Default::default()
        };
        return;
    }

    // Obvious case 3: Current actual sum is 21. Stand!
    if current_hand.get_actual_sum() == 21 {
        let stand_odds = calculate_stand_odds(rule, current_hand, dealer_up_card, current_shoe);

        let stand = {
            if current_hand.is_natural() {
                stand_odds.win * rule.payout_blackjack - stand_odds.lose
            } else {
                stand_odds.win - stand_odds.lose
            }
        };
        ex_stand_hit[current_hand] = Expectation {
            stand,
            ..Default::default()
        };
        return;
    }

    // End of obvious cases. Calculate expectation of Hit using theory of total expectation.
    ex_stand_hit[current_hand] = Expectation {
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

        let (ex_max, _) = get_max_expectation(ex_stand_hit, current_hand, rule);

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
        // Hard sum >= 17
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

        let p = ((original_shoe[card] - dealer_extra_hand[card]) as f64) / current_valid_shoe_total;
        odds[dealer_extra_hand] += &(next_state_odds * p);
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
            payout_insurance: 2.0,
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

        let sol = calculate_solution_with_initial_situation(1, &rule, &initial_situation);
        let mut initial_hand = CardCount::new(&[0; 10]);
        initial_hand.add_card(hand_cards.0);
        initial_hand.add_card(hand_cards.1);
        println!("{:#?}", sol.ex_stand_hit[&initial_hand]);
    }

    #[test]
    #[ignore]
    fn test_calculate_with_unknown_player_cards() {
        let rule = get_typical_rule();
        let mut shoe = CardCount::with_number_of_decks(8);
        let dealer_up_card = 10;
        shoe.remove_card(dealer_up_card);
        let initial_situation = InitialSituation::new(shoe, (0, 0), dealer_up_card);

        let time_start = std::time::SystemTime::now();
        let solution = calculate_solution_with_initial_situation(1, &rule, &initial_situation);
        let no_hand_state = CardCount::with_number_of_decks(0);
        println!("{:#?}", solution.ex_stand_hit[&no_hand_state]);
        println!(
            "{}s",
            std::time::SystemTime::now()
                .duration_since(time_start)
                .unwrap()
                .as_secs_f64()
        );
    }

    #[test]
    #[ignore]
    fn test_calculate_with_unknown_dealer_up_card() {
        let rule = get_typical_rule();
        let shoe = CardCount::with_number_of_decks(8);
        let time_start = std::time::SystemTime::now();
        let sol = calculate_solution_without_initial_situation(1, &rule, &shoe);
        println!("Expectation is {}", sol.ex_total_summary);
        println!(
            "{}s",
            std::time::SystemTime::now()
                .duration_since(time_start)
                .unwrap()
                .as_secs_f64()
        );
    }

    #[test]
    #[ignore]
    fn print_decision_chart_with_known_initial_situations() {
        let rule = get_typical_rule();

        println!("Hard:");
        for my_hand_total in 5..=18 {
            for dealer_up_card in [2, 3, 4, 5, 6, 7, 8, 9, 10, 1] {
                let mut shoe = CardCount::with_number_of_decks(rule.number_of_decks);
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
                    shoe: shoe.clone(),
                    hand_cards,
                    dealer_up_card,
                };

                let sol = calculate_solution_with_initial_situation(1, &rule, &initial_situation);
                let mut initial_hand = CardCount::new(&[0; 10]);
                initial_hand.add_card(hand_cards.0);
                initial_hand.add_card(hand_cards.1);
                let (mut _mx, mut decision) =
                    get_max_expectation(&sol.ex_stand_hit, &initial_hand, &rule);
                if _mx < sol.ex_double {
                    _mx = sol.ex_double;
                    decision = Decision::Double;
                }
                print!("{} ", decision_to_char(decision));
                shoe.add_card(hand_cards.0);
                shoe.add_card(hand_cards.1);
                shoe.add_card(dealer_up_card);
            }
            println!();
        }

        println!();
        println!("Soft:");

        for another_card in 2..=9 {
            for dealer_up_card in [2, 3, 4, 5, 6, 7, 8, 9, 10, 1] {
                let mut shoe = CardCount::with_number_of_decks(rule.number_of_decks);
                let hand_cards = (1, another_card);
                shoe.remove_card(hand_cards.0);
                shoe.remove_card(hand_cards.1);
                shoe.remove_card(dealer_up_card);

                let initial_situation = InitialSituation {
                    shoe: shoe.clone(),
                    hand_cards,
                    dealer_up_card,
                };

                let sol = calculate_solution_with_initial_situation(1, &rule, &initial_situation);
                let mut initial_hand = CardCount::new(&[0; 10]);
                initial_hand.add_card(hand_cards.0);
                initial_hand.add_card(hand_cards.1);
                let (mut _mx, mut decision) =
                    get_max_expectation(&sol.ex_stand_hit, &initial_hand, &rule);
                if _mx < sol.ex_double {
                    _mx = sol.ex_double;
                    decision = Decision::Double;
                }
                print!("{} ", decision_to_char(decision));
                shoe.add_card(hand_cards.0);
                shoe.add_card(hand_cards.1);
                shoe.add_card(dealer_up_card);
            }
            println!();
        }
    }

    #[test]
    #[ignore]
    fn print_decision_chart_without_initial_situation() {
        let rule = get_typical_rule();
        let shoe = CardCount::with_number_of_decks(rule.number_of_decks);

        let sol = calculate_solution_without_initial_situation(3, &rule, &shoe);

        println!("Hard:");
        for my_hand_total in 5..=18 {
            for dealer_up_card in [2, 3, 4, 5, 6, 7, 8, 9, 10, 1] {
                let hand_cards = {
                    if my_hand_total - 2 <= 10 {
                        (2, my_hand_total - 2)
                    } else {
                        (10, my_hand_total - 10)
                    }
                };

                let sol = sol.get_solution_for_initial_situation(hand_cards, dealer_up_card);
                let mut initial_hand = CardCount::new(&[0; 10]);
                initial_hand.add_card(hand_cards.0);
                initial_hand.add_card(hand_cards.1);
                let (mut _mx, mut decision) =
                    get_max_expectation(&sol.ex_stand_hit, &initial_hand, &rule);
                if _mx < sol.ex_double {
                    _mx = sol.ex_double;
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
                let sol = sol.get_solution_for_initial_situation((another_card, 1), dealer_up_card);
                let mut initial_hand = CardCount::new(&[0; 10]);
                initial_hand.add_card(1);
                initial_hand.add_card(another_card);
                let (mut _mx, mut decision) =
                    get_max_expectation(&sol.ex_stand_hit, &initial_hand, &rule);
                if _mx < sol.ex_double {
                    _mx = sol.ex_double;
                    decision = Decision::Double;
                }
                print!("{} ", decision_to_char(decision));
            }
            println!();
        }

        println!();
        println!("Expectation is {}", sol.get_total_expectation());
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
