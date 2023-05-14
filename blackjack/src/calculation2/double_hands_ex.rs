use super::{dealer_ex, ExpectationAll};
use crate::{CardCount, DoubleStateArray, HandState, Rule};

struct DoubleHandsShoePair {
    hand0: CardCount,
    hand_state0: HandState,
    hand1: CardCount,

    dealer_hole_plus_shoe: CardCount,
}

pub struct DoubleHandsPlay<'a> {
    rule: &'a Rule,
    number_of_threads: usize,
    dealer_play: &'a dealer_ex::DealerPlay<'a>,

    solution_double_aux: [DoubleStateArray<ExpectationAll>; 10],
}

impl<'a> DoubleHandsPlay<'a> {
    pub fn new(
        rule: &'a Rule,
        number_of_threads: usize,
        dealer_play: &'a dealer_ex::DealerPlay<'a>,
    ) -> Self {
        Self {
            rule,
            number_of_threads,
            dealer_play,

            solution_double_aux: Default::default(),
        }
    }

    pub fn calculate_expectation_with_double_hands() {}
}
