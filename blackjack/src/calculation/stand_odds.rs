use super::get_card_probability;
use crate::{CardCount, PeekPolicy, Rule, SingleStateArray};
use std::{cmp::Ordering, ops};

#[derive(Clone, Copy, Default, Debug)]
pub struct WinLoseCasesOdds {
    pub win: f64,
    pub push: f64,
    pub lose: f64,
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

pub fn calculate_stand_odds(
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

    let mut odds = SingleStateArray::new();

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
    odds: &mut SingleStateArray<WinLoseCasesOdds>,
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
    use super::super::tests::get_typical_rule;
    use super::*;

    #[test]
    #[ignore]
    fn test_find_win_lose_cases_count() {
        let rule = get_typical_rule();
        let original_shoe = CardCount::new(&[0, 0, 1, 0, 0, 0, 1, 0, 0, 1]);
        let mut dealer_extra_hand = CardCount::new(&[0; 10]);
        let mut odds = SingleStateArray::new();
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
}
