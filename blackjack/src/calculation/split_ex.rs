use crate::{CardCount, Decision, DoubleCardCountIndex, DoubleStateArray, HandState, Rule};
use blackjack_macros::ExpectationAfterSplit;

use super::get_card_probability;

pub trait ExpectationAfterSplit {
    const ALLOW_DAS: bool;
    const ALLOW_LATE_SURRENDER: bool;

    fn stand(&self) -> f64;
    fn hit(&self) -> f64;
    fn double(&self) -> f64;
    fn surrender(&self) -> f64;
    fn set_stand(&mut self, val: f64);
    fn set_hit(&mut self, val: f64);
    fn set_double(&mut self, val: f64);
    fn set_surrender(&mut self, val: f64);

    fn get_max_expectation(&self) -> (f64, Decision) {
        let mut ex = self.stand();
        let mut decision = Decision::Stand;
        if ex < self.hit() {
            ex = self.hit();
            decision = Decision::Hit;
        }
        if Self::ALLOW_DAS {
            if ex < self.double() {
                ex = self.double();
                decision = Decision::Double;
            }
        }
        if Self::ALLOW_LATE_SURRENDER {
            if ex < self.surrender() {
                ex = self.surrender();
                decision = Decision::Surrender;
            }
        }
        (ex, decision)
    }
}

pub fn memoization_calculate_split_expectation<T: ExpectationAfterSplit + Default>(
    // Input parameters
    rule: &Rule,
    dealer_up_card: &u8,
    impossible_dealer_hole_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount,
    current_hand: &CardCount,

    // Output parameters
    ex: &mut DoubleStateArray<T>,
) {
    let card_value = current_hand.get_sum() as u8 / 2;
    let mut current_hand0 = CardCount::with_number_of_decks(0);
    current_hand0.add_card(card_value);
    let mut current_hand1 = current_hand0.clone();

    for card_value0 in 1..=10 {
        current_hand0.add_card(card_value0);
        for card_value1 in 1..=10 {
            current_hand1.add_card(card_value1);

            memoization_calculate_split_expectation_aux0(
                rule,
                dealer_up_card,
                impossible_dealer_hole_card,
                current_shoe,
                &mut current_hand0,
                &mut current_hand1,
                ex,
            );

            current_hand1.remove_card(card_value1);
        }
        current_hand0.remove_card(card_value0);
    }
}

fn memoization_calculate_split_expectation_aux0<T: ExpectationAfterSplit + Default>(
    // Input parameters
    rule: &Rule,
    dealer_up_card: &u8,
    impossible_dealer_hole_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount,
    current_hand0: &mut CardCount,
    current_hand1: &mut CardCount,

    // Output parameters
    ex: &mut DoubleStateArray<T>,
) {
    let state_array_index =
        DoubleCardCountIndex::new(current_hand0, HandState::PlaceHolder, current_hand1);

    if ex.contains_state(state_array_index) {
        return;
    }
    ex[state_array_index] = Default::default();

    // Decision 1: Stand.
    // TODO: Should consider optimization here or not? (When actual sum <= 11, must hit)
    memoization_calculate_split_expectation_aux1(
        rule,
        dealer_up_card,
        impossible_dealer_hole_card,
        current_shoe,
        current_hand0,
        &HandState::Normal,
        current_hand1,
        ex,
    );
    let next_index = DoubleCardCountIndex::new(current_hand0, HandState::Normal, current_hand1);
    let max_ex = ex[next_index].get_max_expectation().0;
    ex[state_array_index].set_stand(max_ex);

    // Obvious cases. In these cases, we can only stand and must stand.
    if current_hand0.bust()
        || current_hand0.get_total() == rule.charlie_number as u16
        || current_hand0.get_actual_sum() == 21
    {
        return;
    }

    // Decision 2: Hit.
    let mut ex_hit = 0.0;
    for card_value in 1..=10 {
        if current_shoe[card_value] == 0 {
            continue;
        }

        current_shoe.remove_card(card_value);
        current_hand0.add_card(card_value);

        memoization_calculate_split_expectation_aux0(
            rule,
            dealer_up_card,
            impossible_dealer_hole_card,
            current_shoe,
            current_hand0,
            current_hand1,
            ex,
        );
        let next_index =
            DoubleCardCountIndex::new(current_hand0, HandState::PlaceHolder, current_hand1);
        let max_ex = ex[next_index].get_max_expectation().0;

        current_hand0.remove_card(card_value);
        current_shoe.add_card(card_value);

        let p = get_card_probability(current_shoe, *impossible_dealer_hole_card, card_value);
        ex_hit += p * max_ex;
    }
    ex[state_array_index].set_hit(ex_hit);

    // TODO: Surrender here
    if T::ALLOW_LATE_SURRENDER {
        memoization_calculate_split_expectation_aux1(
            rule,
            dealer_up_card,
            impossible_dealer_hole_card,
            current_shoe,
            current_hand0,
            &HandState::Surrender,
            current_hand1,
            ex,
        );
        let next_index =
            DoubleCardCountIndex::new(current_hand0, HandState::Surrender, current_hand1);
        let max_ex = ex[next_index].get_max_expectation().0;
        ex[state_array_index].set_surrender(max_ex);
    }

    // TODO: Double here
    if T::ALLOW_DAS && current_hand0.get_total() == 2 {
        let mut ex_double = 0.0;
        for card_value in 1..=10 {
            if current_shoe[card_value] == 0 {
                continue;
            }

            current_shoe.remove_card(card_value);
            current_hand0.add_card(card_value);

            memoization_calculate_split_expectation_aux1(
                rule,
                dealer_up_card,
                impossible_dealer_hole_card,
                current_shoe,
                current_hand0,
                &HandState::Double,
                current_hand1,
                ex,
            );
            let next_index =
                DoubleCardCountIndex::new(current_hand0, HandState::Double, current_hand1);
            let max_ex = ex[next_index].get_max_expectation().0;

            current_hand0.remove_card(card_value);
            current_shoe.add_card(card_value);

            let p = get_card_probability(current_shoe, *impossible_dealer_hole_card, card_value);
            ex_hit += p * max_ex;
        }
        ex[state_array_index].set_double(ex_double);
    }
}

fn memoization_calculate_split_expectation_aux1<T: ExpectationAfterSplit + Default>(
    // Input parameters
    rule: &Rule,
    dealer_up_card: &u8,
    impossible_dealer_hole_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount,
    current_hand0: &mut CardCount,
    hand_state0: &HandState,
    current_hand1: &mut CardCount,

    // Output parameters
    ex: &mut DoubleStateArray<T>,
) {
    let state_array_index =
        DoubleCardCountIndex::new(current_hand0, HandState::PlaceHolder, current_hand1);

    if ex.contains_state(state_array_index) {
        return;
    }
    ex[state_array_index] = Default::default();

    let can_optimize_stand0 =
        current_hand0.bust() || current_hand0.get_total() == rule.charlie_number as u16;
    let can_optimize_stand1 =
        current_hand1.bust() || current_hand1.get_total() == rule.charlie_number as u16;
    if can_optimize_stand0 && can_optimize_stand1 {
        // In this case, we can only stand, and the expectation is independent with dealer cards.
        let mut ex_stand = {
            if current_hand0.bust() {
                -1.0
            } else {
                1.0
            }
        };
        ex_stand += {
            if current_hand1.bust() {
                -1.0
            } else {
                1.0
            }
        };
        ex[state_array_index].set_stand(ex_stand);
        return;
    }

    // Decision 1: Stand. Note that we don't need to consider player Blackjack, because
    // an Ace plus a 10-valued card doesn't count as natural Blackjack in a split hand.

    // Obvious case. In this case, we can only stand and must stand.
    if current_hand1.get_actual_sum() == 21 {
        return;
    }
}

#[derive(Debug, Clone, ExpectationAfterSplit)]
pub struct ExpectationSHDR {
    stand: f64,
    hit: f64,
    double: f64,
    surrender: f64,
}

#[derive(Debug, Clone, ExpectationAfterSplit)]
pub struct ExpectationSHD {
    stand: f64,
    hit: f64,
    double: f64,
}

#[derive(Debug, Clone, ExpectationAfterSplit)]
pub struct ExpectationSHR {
    stand: f64,
    hit: f64,
    surrender: f64,
}

#[derive(Debug, Clone, ExpectationAfterSplit)]
pub struct ExpectationSH {
    stand: f64,
    hit: f64,
}
