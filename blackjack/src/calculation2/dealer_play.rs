use crate::{CardCount, PeekPolicy, Rule, SingleStateArray};

#[derive(Debug, Clone, Default)]
pub struct DealerHandValueProbability {
    // 0 for Bust.
    // [1, 5] for [17, 21].
    // Probability of natural Blackjack = 1.0 - probabilies_prefix_sum[5].
    probabilities_prefix_sum: [f64; 6],
}

impl DealerHandValueProbability {
    pub fn p_worse_than_player(&self, player_actual_sum: u16) -> f64 {
        let x = player_actual_sum as usize;
        match x {
            0..=17 => self.probabilities_prefix_sum[0],
            18..=21 => self.probabilities_prefix_sum[x - 17],
            _ => panic!("Impossible to reach"),
        }
    }

    pub fn p_better_than_player(&self, player_actual_sum: u16) -> f64 {
        let x = player_actual_sum as usize;
        match x {
            0..=16 => 1.0 - self.probabilities_prefix_sum[0],
            17..=21 => 1.0 - self.probabilities_prefix_sum[x - 16],
            _ => panic!("Impossible to reach"),
        }
    }

    fn end_with_bust(&mut self) {
        for p in self.probabilities_prefix_sum.iter_mut() {
            *p = 1.0;
        }
    }

    fn end_with_normal(&mut self, dealer_actual_sum: u16) {
        for i in (dealer_actual_sum - 16) as usize..self.probabilities_prefix_sum.len() {
            self.probabilities_prefix_sum[i] = 1.0;
        }
    }

    fn end_with_natural(&mut self) {}

    fn add_assign_with_p(&mut self, rhs: &Self, p: f64) {
        for i in 0..self.probabilities_prefix_sum.len() {
            self.probabilities_prefix_sum[i] += rhs.probabilities_prefix_sum[i] * p;
        }
    }
}

pub fn dealer_gets_cards(
    // Input parameters
    rule: &Rule,
    dealer_plus_shoe: &CardCount, // Dealer hand cards plus cards in shoe

    // Output parameters
    odds: &mut SingleStateArray<DealerHandValueProbability>,
) {
    let mut dealer_hand = CardCount::with_number_of_decks(0);

    for card_value in 1..=10 {
        dealer_hand.add_card(card_value);
        if odds.contains_state(&dealer_hand) {
            return;
        }

        let memoization_dealer_gets_cards = match card_value {
            1 => match rule.peek_policy {
                PeekPolicy::UpAce | PeekPolicy::UpAceOrTen => memoization_dealer_gets_cards::<10>,
                _ => memoization_dealer_gets_cards::<0>,
            },
            10 => match rule.peek_policy {
                PeekPolicy::UpAceOrTen => memoization_dealer_gets_cards::<1>,
                _ => memoization_dealer_gets_cards::<0>,
            },
            _ => memoization_dealer_gets_cards::<0>,
        };

        memoization_dealer_gets_cards(rule, dealer_plus_shoe, &mut dealer_hand, odds);

        dealer_hand.remove_card(card_value);
    }
}

fn memoization_dealer_gets_cards<const IMPOSSIBLE_DEALER_HOLE_CARD: u8>(
    // Input parameters
    rule: &Rule,
    dealer_plus_shoe: &CardCount, // Dealer hand cards plus cards in shoe

    // Parameters to maintain current state
    dealer_hand: &mut CardCount, // Dealer's hand except for the up card

    // Output parameters
    odds: &mut SingleStateArray<DealerHandValueProbability>,
) {
    if odds.contains_state(dealer_hand) {
        return;
    }
    odds[dealer_hand] = Default::default();

    // Case 1: Dealer must stand.
    if dealer_hand.bust() {
        odds[dealer_hand].end_with_bust();
        return;
    }
    let actual_sum = dealer_hand.get_actual_sum();
    if actual_sum > 17 {
        odds[dealer].end_with_normal(actual_sum);
        return;
    }
    if actual_sum == 17 {
        if !dealer_hand.is_soft() || !rule.dealer_hit_on_soft17 {
            odds[dealer].end_with_normal(17);
            return;
        }
    }

    // Case 2: Dealer must hit.
    let impossible_card_number = {
        if IMPOSSIBLE_DEALER_HOLE_CARD == 0 {
            0
        } else {
            // This is impossible to be 0, because this means that all the cards in the shoe has been
            // dealt, which is impossible to happen.
            dealer_plus_shoe[IMPOSSIBLE_DEALER_HOLE_CARD] - dealer_hand[IMPOSSIBLE_DEALER_HOLE_CARD]
        }
    };
    let current_valid_shoe_total =
        dealer_plus_shoe.get_total() - dealer_hand.get_total() - impossible_card_number;
    let current_valid_shoe_total = current_valid_shoe_total as f64;

    let (next_card_min, next_card_max) = match IMPOSSIBLE_DEALER_HOLE_CARD {
        0 => (1, 10),
        1 => (2, 10),
        10 => (1, 9),
        _ => panic!("Impossible to reach"),
    };

    for card_value in next_card_min..=next_card_max {
        if dealer_hand[card_value] == dealer_plus_shoe[card_value] {
            continue;
        }

        dealer_plus_shoe.add_card(card_value);

        memoization_dealer_gets_cards::<0>(rule, dealer_plus_shoe, dealer_hand, odds);
        let next_state_odds = &odds[dealer_hand] as *const DealerHandValueProbability;

        dealer_plus_shoe.remove_card(card_value);

        let target_cards_in_shoe = dealer_plus_shoe[card_value] - dealer_hand[card_value];
        let p = target_cards_in_shoe as f64 / current_valid_shoe_total;
        unsafe {
            // Here, we know that we are referencing 2 different pieces of memory, but
            // compilier doesn't know. This is absolutely safe.
            odds[dealer_hand].add_assign_with_p(&*next_state_odds, p);
        }
    }
}
