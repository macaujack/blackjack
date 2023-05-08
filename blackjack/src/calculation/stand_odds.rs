use crate::{CardCount, Rule, SingleStateArray};

/// Note that the callers of `end_with_dealer_*` functions must handle the following cases separately before
/// calling this function:
/// 1. Player's hand busts.
/// 2. Player's hand reaches Charlie number.
/// 3. Player gets a natural Blackjack.
pub trait DealerHandHandler {
    fn end_with_dealer_bust(&mut self);
    fn end_with_dealer_normal(&mut self, dealer_actual_sum: u16, player_actual_sum: u16);
    fn end_with_dealer_natural(&mut self);

    fn add_assign_with_p(&mut self, rhs: &Self, p: f64);
}

/// Note that the callers of this function must ensure that if player_sum is 21, it must NOT be
/// a natural Blackjack. Player natural Blackjack should be handled separately as a special
/// case before recursively calling this function.
pub fn memoization_find_win_lose_odds<
    T: Default + DealerHandHandler,
    const NEXT_CARD_MIN: u8,
    const NEXT_CARD_MAX: u8,
>(
    // Input parameters
    rule: &Rule,
    player_sum: &u16,
    dealer_up_card: &u8,
    original_shoe: &CardCount, // Original cards in the shoe just before dealer's hole card is revealed

    // Parameters to maintain current state
    dealer_extra_hand: &mut CardCount, // Dealer's hand except for the up card
    odds: &mut SingleStateArray<T>,
) {
    if odds.contains_state(dealer_extra_hand) {
        return;
    }
    odds[dealer_extra_hand] = Default::default();

    // Case 1: Dealer must stand.
    let dealer_sum = dealer_extra_hand.get_sum() + (*dealer_up_card as u16);
    let is_soft = dealer_extra_hand.is_soft() || *dealer_up_card == 1;
    if dealer_sum > 21 {
        odds[dealer_extra_hand].end_with_dealer_bust();
        return;
    }
    if dealer_sum >= 17 {
        // Hard sum >= 17
        odds[dealer_extra_hand].end_with_dealer_normal(dealer_sum, *player_sum);
        return;
    }
    if is_soft {
        // Dealer gets natural Blackjack!! OMG!!
        // Note that if the peek policy is UpAceOrTen, dealer will peek the hole card when the up card is Ace or 10,
        // which immediately ends the game if she gets a natural Blackjack. This in turn makes the following 'if'
        // impossible to run.
        if dealer_sum + 10 == 21 && dealer_extra_hand.get_total() == 1 {
            odds[dealer_extra_hand].end_with_dealer_natural();
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
            odds[dealer_extra_hand].end_with_dealer_normal(dealer_sum + 10, *player_sum);
            return;
        }
    }

    // Case 2: Dealer must hit.
    let current_valid_shoe_total = original_shoe.get_total() - dealer_extra_hand.get_total() - {
        if NEXT_CARD_MAX < 10 {
            original_shoe[10]
        } else if NEXT_CARD_MIN > 1 {
            original_shoe[1]
        } else {
            0
        }
    };
    let current_valid_shoe_total = current_valid_shoe_total as f64;

    for card in NEXT_CARD_MIN..=NEXT_CARD_MAX {
        if dealer_extra_hand[card] == original_shoe[card] {
            continue;
        }

        dealer_extra_hand.add_card(card);
        memoization_find_win_lose_odds::<T, 1, 10>(
            rule,
            player_sum,
            dealer_up_card,
            original_shoe,
            dealer_extra_hand,
            odds,
        );
        let next_state_odds = &odds[dealer_extra_hand] as *const T;
        dealer_extra_hand.remove_card(card);

        let p = ((original_shoe[card] - dealer_extra_hand[card]) as f64) / current_valid_shoe_total;
        unsafe {
            // Here, we know that we are referencing 2 different pieces of memory, but
            // compilier doesn't know.
            odds[dealer_extra_hand].add_assign_with_p(&*next_state_odds, p);
        }
    }
}
