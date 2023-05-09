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
pub fn memoization_dealer_get_cards<
    T: Default + DealerHandHandler,
    const NEXT_CARD_MIN: u8,
    const NEXT_CARD_MAX: u8,
>(
    // Input parameters
    rule: &Rule,
    player_sum: &u16,
    dealer_up_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount, // Current shoe, propably including dealer hole card.
    dealer_extra_hand: &mut CardCount, // Dealer's hand except for the up card
    odds: &mut SingleStateArray<T>,
) {
    if odds.contains_state(&current_shoe) {
        return;
    }
    odds[current_shoe] = Default::default();

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
    let current_valid_shoe_total = current_shoe.get_total() - {
        if NEXT_CARD_MAX < 10 {
            current_shoe[10]
        } else if NEXT_CARD_MIN > 1 {
            current_shoe[1]
        } else {
            0
        }
    };
    let current_valid_shoe_total = current_valid_shoe_total as f64;

    for card in NEXT_CARD_MIN..=NEXT_CARD_MAX {
        if current_shoe[card] == 0 {
            continue;
        }

        current_shoe.remove_card(card);
        dealer_extra_hand.add_card(card);
        memoization_dealer_get_cards::<T, 1, 10>(
            rule,
            player_sum,
            dealer_up_card,
            current_shoe,
            dealer_extra_hand,
            odds,
        );
        let next_state_odds = &odds[dealer_extra_hand] as *const T;
        dealer_extra_hand.remove_card(card);
        current_shoe.add_card(card);

        let p = (current_shoe[card] as f64) / current_valid_shoe_total;
        unsafe {
            // Here, we know that we are referencing 2 different pieces of memory, but
            // compilier doesn't know.
            odds[dealer_extra_hand].add_assign_with_p(&*next_state_odds, p);
        }
    }
}
