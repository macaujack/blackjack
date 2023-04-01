use super::{Decision, Rule};
use crate::{CardCount, InitialSituation, StateArray};
use std::{cmp::Ordering, ops};

const PERM_SIZE: usize = 500;
static PERM: [[u128; PERM_SIZE]; PERM_SIZE] = get_perm();

/// Gets the lookup table of permutatiions. It is OK to ignore the integer type
/// overflow, because in our case, we only need some small numbers.
const fn get_perm() -> [[u128; PERM_SIZE]; PERM_SIZE] {
    let mut ret: [[u128; PERM_SIZE]; PERM_SIZE] = [[0; PERM_SIZE]; PERM_SIZE];

    let mut i = 0;
    let mut j;
    let mut cur: u128;
    while i < ret.len() {
        ret[i][0] = 1;
        j = 1;
        cur = i as u128;
        while j <= i {
            ret[i][j] = ret[i][j - 1] * cur;
            if ret[i][j] >= 1 << 110 {
                ret[i][j] = 0xffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff;
                break;
            }
            j += 1;
            cur -= 1;
        }
        i += 1;
    }

    ret
}

#[derive(Clone, Copy, Debug)]
pub struct MaxExpectation {
    pub hit: f64,
    pub stand: f64,
    pub double: f64,
}

impl MaxExpectation {
    fn get_max_expectation(&self) -> (f64, Decision) {
        let mut max_ex = -0.5;
        let mut decision = Decision::Surrender;
        if max_ex < self.hit {
            max_ex = self.hit;
            decision = Decision::Hit;
        }
        if max_ex < self.stand {
            max_ex = self.stand;
            decision = Decision::Stand;
        }
        if max_ex < self.double {
            max_ex = self.double;
            decision = Decision::Double;
        }

        (max_ex, decision)
    }
}

impl Default for MaxExpectation {
    fn default() -> Self {
        MaxExpectation {
            hit: -f64::INFINITY,
            stand: -f64::INFINITY,
            double: -f64::INFINITY,
        }
    }
}

pub struct SolutionForInitialSituation {
    /// Note that this doesn't take the following cases into consideration:
    /// 1. Split pairs
    /// 2. Buy insurance
    /// 3. Blackjack (both for player and dealer)
    pub general_solution: StateArray<MaxExpectation>,
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

    // TODO: Calculate the expectation when able to split.
    SolutionForInitialSituation {
        general_solution,
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
    // Obvious case 2: Current hand sum is 21. Stand!
    if current_sum == 21 || current_hand.is_soft() && current_sum == 11 {
        // Stand (obvious)
        solution[current_hand] = MaxExpectation {
            stand: calculate_stand_expectation(rule, current_hand, dealer_up_card, current_shoe),
            ..Default::default()
        };
        return;
    }

    solution[current_hand] = MaxExpectation {
        hit: 0.0,
        double: 0.0,
        ..Default::default()
    };

    let total_shoe_count = current_shoe.get_total() as f64;

    for i in 1..=10 {
        if current_shoe[i] == 0 {
            continue;
        }

        current_shoe.remove_card(i);
        current_hand.add_card(i);

        memoization_find_solution(rule, dealer_up_card, current_shoe, current_hand, solution);

        let (ex_max, _): (f64, _) = solution[current_hand].get_max_expectation();
        let ex_stand: f64 = solution[current_hand].stand;

        current_hand.remove_card(i);
        current_shoe.add_card(i);

        let p = (current_shoe[i] as f64) / total_shoe_count;
        solution[current_hand].hit += p * ex_max;
        solution[current_hand].double += p * 2.0 * ex_stand;
    }

    solution[current_hand].stand =
        calculate_stand_expectation(rule, current_hand, dealer_up_card, current_shoe);
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

fn calculate_stand_expectation(
    rule: &Rule,
    player_hand: &CardCount,
    dealer_up_card: &u8,
    shoe: &CardCount,
) -> f64 {
    let mut dealer_extra_hand = CardCount::new(&[0; 10]);
    let player_sum = {
        if player_hand.is_soft() && player_hand.get_sum() + 10 <= 21 {
            player_hand.get_sum() + 10
        } else {
            player_hand.get_sum()
        }
    };

    let mut odds = StateArray::new();

    memoization_find_win_lose_cases_count(
        rule,
        &player_sum,
        dealer_up_card,
        &shoe,
        &mut dealer_extra_hand,
        &mut odds,
    );

    odds[&dealer_extra_hand].win - odds[&dealer_extra_hand].lose
}

fn memoization_find_win_lose_cases_count(
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
    let p = 1.0
        / (PERM[original_shoe.get_total() as usize][dealer_extra_hand.get_total() as usize] as f64);
    if dealer_sum > 21 {
        odds[dealer_extra_hand] = WinLoseCasesOdds {
            win: p,
            ..Default::default()
        };
        return;
    }
    if dealer_sum >= 17 {
        add_to_win_lose_cases_count(*player_sum, dealer_sum, &mut odds[dealer_extra_hand], p);
        return;
    }
    if is_soft && rule.dealer_hit_on_soft17 && dealer_sum + 10 > 17 && dealer_sum + 10 <= 21 {
        add_to_win_lose_cases_count(
            *player_sum,
            dealer_sum + 10,
            &mut odds[dealer_extra_hand],
            p,
        );
        return;
    }

    // Case 2: Dealer must hit.
    for card in 1..=10 {
        if dealer_extra_hand[card] == original_shoe[card] {
            continue;
        }

        dealer_extra_hand.add_card(card);
        memoization_find_win_lose_cases_count(
            rule,
            player_sum,
            dealer_up_card,
            original_shoe,
            dealer_extra_hand,
            odds,
        );
        let next_state_odds = odds[dealer_extra_hand];
        dealer_extra_hand.remove_card(card);

        odds[dealer_extra_hand] +=
            &(next_state_odds * ((original_shoe[card] - dealer_extra_hand[card]) as f64));
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
            dealer_hit_on_soft17: true,
            allow_das: true,
            allow_late_surrender: true,
            dealer_peek_hole_card: true,

            payout_blackjack: 1.5,
            payout_insurance: 0.0,
        }
    }

    #[test]
    fn test_find_win_lose_cases_count() {
        let rule = get_typical_rule();
        let original_shoe = CardCount::new(&[20, 20, 19, 19, 20, 20, 20, 20, 20, 127]);
        let mut dealer_extra_hand = CardCount::new(&[0; 10]);
        let mut odds = StateArray::new();
        memoization_find_win_lose_cases_count(
            &rule,
            &14,
            &3,
            &original_shoe,
            &mut dealer_extra_hand,
            &mut odds,
        );

        let od = odds[&CardCount::new(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0])];
        println!("{:#?}", od);
        println!("{:#?}", od.win + od.push + od.lose);
    }

    #[test]
    fn test_decision() {
        let rule = get_typical_rule();

        let mut counts = [4 * (rule.number_of_decks as u16); 10];
        counts[9] = 16 * (rule.number_of_decks as u16);
        let mut shoe = CardCount::new(&counts);
        let hand_cards = (2, 8);
        let dealer_up_card = 8;
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
}
