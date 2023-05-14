mod dealer_ex;
mod double_hands_ex;
mod util;

use crate::Decision;

#[derive(Debug, Clone)]
pub struct ExpectationAll {
    pub hit: f64,
    pub stand: f64,
    pub double: f64,
    pub surrender: f64,
    pub split: f64,

    pub insurance: f64,

    pub summary: f64,
    pub decision: Decision,
}

impl Default for ExpectationAll {
    fn default() -> Self {
        Self {
            hit: -f64::INFINITY,
            stand: -f64::INFINITY,
            double: -f64::INFINITY,
            surrender: -f64::INFINITY,
            split: -f64::INFINITY,

            insurance: -f64::INFINITY,

            summary: -f64::INFINITY,
            decision: Decision::PlaceHolder,
        }
    }
}

impl ExpectationAll {
    pub fn get_max_expectation(&self) -> (f64, Decision) {
        let (mut mx_ex, mut decision) = (self.hit, Decision::Hit);
        if mx_ex < self.stand {
            (mx_ex, decision) = (self.stand, Decision::Stand);
        }
        if mx_ex < self.double {
            (mx_ex, decision) = (self.double, Decision::Double);
        }
        if mx_ex < self.surrender {
            (mx_ex, decision) = (self.surrender, Decision::Surrender);
        }
        if mx_ex < self.split {
            (mx_ex, decision) = (self.split, Decision::Split);
        }

        (mx_ex, decision)
    }
}

use crate::{CardCount, DoubleCardCountIndex, DoubleStateArray, HandState, Rule, SingleStateArray};

pub struct PlayerPlay<'a> {
    rule: &'a Rule,
    number_of_threads: usize,

    dealer_play: dealer_ex::DealerPlay<'a>,
    zero_card_count_aux: CardCount,
    card_counts_aux: Vec<CardCount>,

    solution_single_aux: [SingleStateArray<ExpectationAll>; 10],
}

impl<'a> PlayerPlay<'a> {
    pub fn new(rule: &'a Rule, number_of_threads: usize) -> Self {
        Self {
            rule,
            number_of_threads,

            dealer_play: dealer_ex::DealerPlay::new(rule, number_of_threads),
            zero_card_count_aux: CardCount::with_number_of_decks(0),
            card_counts_aux: Vec::new(),

            solution_single_aux: Default::default(),
        }
    }

    pub fn solve<'b>(&'b mut self, shoe: &CardCount) {
        for i in 0..10 {
            self.solution_single_aux[i].clear();
        }
        let mut shoe = shoe.clone();

        // Step 1: Update dealer's expectations.
        self.get_valid_shoes_after_stand(&mut shoe);
        self.dealer_play.update_dealer_odds(&self.card_counts_aux);

        // Step 2: Calculate solution with 2 hands.

        // Step 3: Calculate solution with 1 hand.
    }

    pub fn get_expectation_with_single(
        &mut self,
        hand: &CardCount,
        dealer_up_card: u8,
    ) -> &ExpectationAll {
        &self.solution_single_aux[(dealer_up_card - 1) as usize][hand]
    }

    fn get_valid_shoes_after_stand(&mut self, shoe: &mut CardCount) {
        // TODO: Consider double hands
        let charlie_number = self.rule.charlie_number as u16;
        self.card_counts_aux.clear();
        Self::get_valid_shoes_after_stand_aux(
            &charlie_number,
            &mut self.zero_card_count_aux,
            shoe,
            1,
            &mut self.card_counts_aux,
        );
    }

    /****************************************************************/
    /****************************************************************/
    /****************************************************************/
    /**************** Begin of calculation functions ****************/
    /****************************************************************/
    /****************************************************************/
    /****************************************************************/

    fn get_valid_shoes_after_stand_aux(
        charlie_number: &u16,
        current_hand: &mut CardCount,
        current_shoe: &mut CardCount,
        loop_start_card: u8,

        result: &mut Vec<CardCount>,
    ) {
        result.push(current_shoe.clone());

        if current_hand.get_sum() >= 21 || current_hand.get_total() == *charlie_number {
            return;
        }

        for next_card_value in loop_start_card..=10 {
            if current_shoe[next_card_value] == 0 {
                continue;
            }

            current_shoe.remove_card(next_card_value);
            current_hand.add_card(next_card_value);

            Self::get_valid_shoes_after_stand_aux(
                charlie_number,
                current_hand,
                current_shoe,
                next_card_value,
                result,
            );

            current_hand.remove_card(next_card_value);
            current_shoe.add_card(next_card_value);
        }
    }
}
