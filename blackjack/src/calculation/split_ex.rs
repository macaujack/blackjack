use crate::{
    CardCount, Decision, DoubleCardCountIndex, DoubleStateArray, HandState, Rule, SingleStateArray,
};
use blackjack_macros::ExpectationAfterSplit;

use super::{
    get_card_probability,
    stand_odds::{memoization_dealer_get_cards, DealerHandHandler},
};

pub static mut R0_BEF: u64 = 0;
pub static mut R0_AFT: u64 = 0;
pub static mut R1_BEF: u64 = 0;
pub static mut R1_AFT: u64 = 0;
pub static mut R2_BEF: u64 = 0;
pub static mut R2_AFT: u64 = 0;

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

pub fn calculate_split_expectation<T: ExpectationAfterSplit + Default>(
    // Input parameters
    rule: &Rule,
    dealer_up_card: &u8,
    impossible_dealer_hole_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount,
    current_hand: &CardCount,

    // Output parameters
    ex: &mut DoubleStateArray<T>,
    dealer_hand_ps: &mut SingleStateArray<SingleStateArray<DealerHandValueProbability>>,
) {
    let split_card_value = current_hand.get_sum() as u8 / 2;
    let mut current_hand0 = CardCount::with_number_of_decks(0);
    current_hand0.add_card(split_card_value);
    let mut current_hand1 = current_hand0.clone();

    // Special case: Split Aces
    if split_card_value == 1 && !rule.allow_decisions_after_split_aces {
        calculate_expectation_double_aces(
            rule,
            dealer_up_card,
            impossible_dealer_hole_card,
            current_shoe,
            &mut current_hand0,
            &mut current_hand1,
            ex,
            dealer_hand_ps,
        );
        return;
    }

    let memoization_calculate_split_expectation_aux0 = match *impossible_dealer_hole_card {
        0 => memoization_calculate_split_expectation_aux0::<T, 1, 10>,
        1 => memoization_calculate_split_expectation_aux0::<T, 2, 10>,
        10 => memoization_calculate_split_expectation_aux0::<T, 1, 9>,
        _ => panic!("Impossible to reach"),
    };

    memoization_calculate_split_expectation_aux0(
        rule,
        dealer_up_card,
        impossible_dealer_hole_card,
        current_shoe,
        &mut current_hand0,
        &mut current_hand1,
        ex,
        dealer_hand_ps,
    );
}

fn calculate_expectation_double_aces<T: ExpectationAfterSplit + Default>(
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
    dealer_hand_ps: &mut SingleStateArray<SingleStateArray<DealerHandValueProbability>>,
) {
    let state_array_index =
        DoubleCardCountIndex::new(current_hand0, HandState::PlaceHolder, current_hand1);
    if ex.contains_state(state_array_index) {
        return;
    }
    ex[state_array_index] = Default::default();

    let memoization_calculate_split_expectation_aux2 = match *impossible_dealer_hole_card {
        0 => memoization_calculate_split_expectation_aux2::<T, 1, 10, 1>,
        1 => memoization_calculate_split_expectation_aux2::<T, 2, 10, 1>,
        10 => memoization_calculate_split_expectation_aux2::<T, 1, 9, 1>,
        _ => panic!("Impossible to reach"),
    };
    let mut ex_hit = 0.0;

    for card_value0 in 1..=10 {
        let p = get_card_probability(current_shoe, *impossible_dealer_hole_card, card_value0);
        if p == 0.0 {
            continue;
        }
        current_shoe.remove_card(card_value0);
        current_hand0.add_card(card_value0);

        for card_value1 in 1..=card_value0 {
            let mut p =
                p * get_card_probability(current_shoe, *impossible_dealer_hole_card, card_value1);
            if p == 0.0 {
                continue;
            }
            // TODO: Prove that this is correct.
            if card_value0 != card_value1 {
                p *= 2.0;
            }
            let p = p;

            current_shoe.remove_card(card_value1);
            current_hand1.add_card(card_value1);

            memoization_calculate_split_expectation_aux2(
                rule,
                dealer_up_card,
                current_shoe,
                current_hand0,
                &HandState::Normal,
                current_hand1,
                ex,
                dealer_hand_ps,
            );

            let next_index =
                DoubleCardCountIndex::new(current_hand0, HandState::Normal, current_hand1);
            ex_hit += p * ex[next_index].stand();

            current_hand1.remove_card(card_value1);
            current_shoe.add_card(card_value1);
        }

        current_hand0.remove_card(card_value0);
        current_shoe.add_card(card_value0);
    }
    ex[state_array_index].set_hit(ex_hit);
}

fn memoization_calculate_split_expectation_aux0<
    T: ExpectationAfterSplit + Default,
    const DEALER_HOLE_CARD_MIN: u8,
    const DEALER_HOLE_CARD_MAX: u8,
>(
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
    dealer_hand_ps: &mut SingleStateArray<SingleStateArray<DealerHandValueProbability>>,
) {
    let state_array_index =
        DoubleCardCountIndex::new(current_hand0, HandState::PlaceHolder, current_hand1);

    unsafe {
        R0_BEF += 1;
    }
    if ex.contains_state(state_array_index) {
        return;
    }
    unsafe {
        R0_AFT += 1;
    }
    ex[state_array_index] = Default::default();

    // Decision 1: Stand.
    // Optimization here. When actual_sum <= 11, we cannot Stand.
    // TODO: Prove this is correct.
    // Is this correct? Since there may be some cases, where we Stand even if actual_sum <= 11.
    // This is to make the second hand more probable to get a better result.
    if current_hand0.get_actual_sum() > 11 {
        memoization_calculate_split_expectation_aux1::<T, DEALER_HOLE_CARD_MIN, DEALER_HOLE_CARD_MAX>(
            rule,
            dealer_up_card,
            impossible_dealer_hole_card,
            current_shoe,
            current_hand0,
            &HandState::Normal,
            current_hand1,
            ex,
            dealer_hand_ps,
        );
        let next_index = DoubleCardCountIndex::new(current_hand0, HandState::Normal, current_hand1);
        let max_ex = ex[next_index].get_max_expectation().0;
        ex[state_array_index].set_stand(max_ex);
    } else {
        ex[state_array_index].set_stand(-f64::INFINITY);
    }

    // Obvious cases. In these cases, we can only stand and must stand. Since we have calculated the expectation
    // of Stand in previous code, we simply return here.
    if current_hand0.bust()
        || current_hand0.get_total() == rule.charlie_number as u16
        || current_hand0.get_actual_sum() == 21
    {
        return;
    }

    // Decision 2: Hit.
    let mut ex_hit = 0.0;
    for card_value in 1..=10 {
        let p = get_card_probability(current_shoe, *impossible_dealer_hole_card, card_value);
        if p == 0.0 {
            continue;
        }

        current_shoe.remove_card(card_value);
        current_hand0.add_card(card_value);

        memoization_calculate_split_expectation_aux0::<T, 1, 10>(
            rule,
            dealer_up_card,
            impossible_dealer_hole_card,
            current_shoe,
            current_hand0,
            current_hand1,
            ex,
            dealer_hand_ps,
        );
        let next_index =
            DoubleCardCountIndex::new(current_hand0, HandState::PlaceHolder, current_hand1);
        let max_ex = ex[next_index].get_max_expectation().0;

        current_hand0.remove_card(card_value);
        current_shoe.add_card(card_value);

        ex_hit += p * max_ex;
    }
    ex[state_array_index].set_hit(ex_hit);

    // Decision 3: Surrender.
    if T::ALLOW_LATE_SURRENDER {
        memoization_calculate_split_expectation_aux1::<T, DEALER_HOLE_CARD_MIN, DEALER_HOLE_CARD_MAX>(
            rule,
            dealer_up_card,
            impossible_dealer_hole_card,
            current_shoe,
            current_hand0,
            &HandState::Surrender,
            current_hand1,
            ex,
            dealer_hand_ps,
        );
        let next_index =
            DoubleCardCountIndex::new(current_hand0, HandState::Surrender, current_hand1);
        let max_ex = ex[next_index].get_max_expectation().0;
        ex[state_array_index].set_surrender(max_ex);
    }

    // Decision 4: Double.
    if T::ALLOW_DAS && current_hand0.get_total() == 2 {
        let mut ex_double = 0.0;
        for card_value in 1..=10 {
            let p = get_card_probability(current_shoe, *impossible_dealer_hole_card, card_value);
            if p == 0.0 {
                continue;
            }

            current_shoe.remove_card(card_value);
            current_hand0.add_card(card_value);

            memoization_calculate_split_expectation_aux1::<
                T,
                DEALER_HOLE_CARD_MIN,
                DEALER_HOLE_CARD_MAX,
            >(
                rule,
                dealer_up_card,
                impossible_dealer_hole_card,
                current_shoe,
                current_hand0,
                &HandState::Double,
                current_hand1,
                ex,
                dealer_hand_ps,
            );
            let next_index =
                DoubleCardCountIndex::new(current_hand0, HandState::Double, current_hand1);
            let max_ex = ex[next_index].get_max_expectation().0;

            current_hand0.remove_card(card_value);
            current_shoe.add_card(card_value);

            ex_double += p * max_ex;
        }
        ex[state_array_index].set_double(ex_double);
    }
}

fn memoization_calculate_split_expectation_aux1<
    T: ExpectationAfterSplit + Default,
    const DEALER_HOLE_CARD_MIN: u8,
    const DEALER_HOLE_CARD_MAX: u8,
>(
    // Input parameters
    rule: &Rule,
    dealer_up_card: &u8,
    impossible_dealer_hole_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount,
    current_hand0: &CardCount,
    hand_state0: &HandState,
    current_hand1: &mut CardCount,

    // Output parameters
    ex: &mut DoubleStateArray<T>,
    dealer_hand_ps: &mut SingleStateArray<SingleStateArray<DealerHandValueProbability>>,
) {
    let state_array_index = DoubleCardCountIndex::new(current_hand0, *hand_state0, current_hand1);

    unsafe {
        R1_BEF += 1;
    }
    if ex.contains_state(state_array_index) {
        return;
    }
    unsafe {
        R1_AFT += 1;
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
    if current_hand1.get_actual_sum() > 11 {
        memoization_calculate_split_expectation_aux2::<
            T,
            DEALER_HOLE_CARD_MIN,
            DEALER_HOLE_CARD_MAX,
            1,
        >(
            rule,
            dealer_up_card,
            current_shoe,
            current_hand0,
            hand_state0,
            current_hand1,
            ex,
            dealer_hand_ps,
        );
    }
    // Obvious case. In this case, we can only stand and must stand.
    if can_optimize_stand1 || current_hand1.get_actual_sum() == 21 {
        return;
    }

    // Decision 2: Hit.
    let mut ex_hit = 0.0;
    for card_value in 1..=10 {
        let p = get_card_probability(current_shoe, *impossible_dealer_hole_card, card_value);
        if p == 0.0 {
            continue;
        }

        current_shoe.remove_card(card_value);
        current_hand1.add_card(card_value);

        memoization_calculate_split_expectation_aux1::<T, 1, 10>(
            rule,
            dealer_up_card,
            impossible_dealer_hole_card,
            current_shoe,
            current_hand0,
            hand_state0,
            current_hand1,
            ex,
            dealer_hand_ps,
        );
        let next_index = DoubleCardCountIndex::new(current_hand0, *hand_state0, current_hand1);
        let max_ex = ex[next_index].get_max_expectation().0;

        current_hand1.remove_card(card_value);
        current_shoe.add_card(card_value);

        ex_hit += p * max_ex;
    }
    ex[state_array_index].set_hit(ex_hit);

    // Decision 3: Surrender
    if T::ALLOW_LATE_SURRENDER {
        memoization_calculate_split_expectation_aux2::<
            T,
            DEALER_HOLE_CARD_MIN,
            DEALER_HOLE_CARD_MAX,
            2,
        >(
            rule,
            dealer_up_card,
            current_shoe,
            current_hand0,
            hand_state0,
            current_hand1,
            ex,
            dealer_hand_ps,
        );
    }

    // Decision 4: Double.
    if T::ALLOW_DAS && current_hand1.get_total() == 2 {
        let mut ex_double = 0.0;
        for card_value in 1..=10 {
            let p = get_card_probability(current_shoe, *impossible_dealer_hole_card, card_value);
            if p == 0.0 {
                continue;
            }

            current_shoe.remove_card(card_value);
            current_hand1.add_card(card_value);

            memoization_calculate_split_expectation_aux2::<
                T,
                DEALER_HOLE_CARD_MIN,
                DEALER_HOLE_CARD_MAX,
                3,
            >(
                rule,
                dealer_up_card,
                current_shoe,
                current_hand0,
                hand_state0,
                current_hand1,
                ex,
                dealer_hand_ps,
            );
            let next_index = DoubleCardCountIndex::new(current_hand0, *hand_state0, current_hand1);
            let max_ex = ex[next_index].get_max_expectation().0;

            current_hand1.remove_card(card_value);
            current_shoe.add_card(card_value);

            ex_double += p * max_ex;
        }
        ex[state_array_index].set_double(ex_double);
    }
}

fn memoization_calculate_split_expectation_aux2<
    T: ExpectationAfterSplit + Default,
    const DEALER_HOLE_CARD_MIN: u8,
    const DEALER_HOLE_CARD_MAX: u8,
    // 1 for HandState::Normal
    // 2 for HandState::Surrender
    // 3 for HandState::Double
    const HAND_STATE1: u8,
>(
    // Input parameters
    rule: &Rule,
    dealer_up_card: &u8,

    // Parameters to maintain current state
    current_shoe: &mut CardCount,
    current_hand0: &CardCount,
    hand_state0: &HandState,
    current_hand1: &CardCount,

    // Output parameters
    ex: &mut DoubleStateArray<T>,
    dealer_hand_ps: &mut SingleStateArray<SingleStateArray<DealerHandValueProbability>>,
) {
    let state_array_index = DoubleCardCountIndex::new(current_hand0, *hand_state0, current_hand1);

    let mut dealer_extra_hand = CardCount::with_number_of_decks(0);
    unsafe {
        R2_BEF += 1;
    }
    if !dealer_hand_ps.contains_state(current_shoe) {
        unsafe {
            R2_AFT += 1;
        }
        dealer_hand_ps[current_shoe] = Default::default();
        memoization_dealer_get_cards::<
            DealerHandValueProbability,
            DEALER_HOLE_CARD_MIN,
            DEALER_HOLE_CARD_MAX,
        >(
            rule,
            &0,
            dealer_up_card,
            current_shoe,
            &mut dealer_extra_hand,
            &mut dealer_hand_ps[current_shoe],
        );
    }

    let hand_state1 = &match HAND_STATE1 {
        1 => HandState::Normal,
        2 => HandState::Surrender,
        3 => HandState::Double,
        _ => panic!("Impossible to reach"),
    };

    let dealer_odds = &dealer_hand_ps[current_shoe][&dealer_extra_hand];
    let ex0 = calculate_expectation_for_one_hand(rule, current_hand0, hand_state0, dealer_odds);
    let ex1 = calculate_expectation_for_one_hand(rule, current_hand1, hand_state1, dealer_odds);
    let ex_sum = ex0 + ex1;
    match HAND_STATE1 {
        1 => ex[state_array_index].set_stand(ex_sum),
        2 => ex[state_array_index].set_surrender(ex_sum),
        3 => ex[state_array_index].set_double(ex_sum),
        _ => panic!("Impossible to reach"),
    }
}

fn calculate_expectation_for_one_hand(
    rule: &Rule,
    hand_cards: &CardCount,
    hand_state: &HandState,
    dealer_odds: &DealerHandValueProbability,
) -> f64 {
    if *hand_state == HandState::Surrender {
        return -0.5;
    }
    if hand_cards.bust() {
        return -1.0;
    }
    if hand_cards.get_total() == rule.charlie_number as u16 {
        return 1.0;
    }
    let player_actual_sum = hand_cards.get_actual_sum();
    let p_win: f64 = dealer_odds.p_worse_than_player(player_actual_sum);
    let p_lose: f64 = dealer_odds.p_better_than_player(player_actual_sum);
    let mut ret = p_win - p_lose;

    if *hand_state == HandState::Double {
        ret *= 2.0;
    }

    ret
}

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
}

impl DealerHandHandler for DealerHandValueProbability {
    fn end_with_dealer_bust(&mut self) {
        for p in self.probabilities_prefix_sum.iter_mut() {
            *p = 1.0;
        }
    }

    fn end_with_dealer_normal(&mut self, dealer_actual_sum: u16, _: u16) {
        for i in (dealer_actual_sum - 16) as usize..self.probabilities_prefix_sum.len() {
            self.probabilities_prefix_sum[i] = 1.0;
        }
    }

    fn end_with_dealer_natural(&mut self) {}

    fn add_assign_with_p(&mut self, rhs: &Self, p: f64) {
        for i in 0..self.probabilities_prefix_sum.len() {
            self.probabilities_prefix_sum[i] += rhs.probabilities_prefix_sum[i] * p;
        }
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
