use crate::{
    calculation::{
        calculate_solution_with_initial_situation, get_max_expectation, Expectation,
        SolutionForInitialSituation,
    },
    CardCount, Decision, InitialSituation, Rule, StateArray,
};

pub trait Strategy {
    fn new(rule: &Rule) -> Self;
    fn init_with_initial_situation(&mut self, rule: &Rule, initial_situation: &InitialSituation);
    fn make_decision(
        &mut self,
        rule: &Rule,
        current_hand: &CardCount,
        current_split_all_times: u8,
        current_split_ace_times: u8,
    ) -> Decision;
}

pub struct BasicStrategy {
    dealer_up_card: u8,
    hard_charts: [[(Decision, Decision); 10]; 14],
    soft_charts: [[(Decision, Decision); 10]; 9],
    pair_charts: [[(Decision, Decision); 10]; 10],
}

impl Strategy for BasicStrategy {
    fn new(rule: &Rule) -> BasicStrategy {
        // TODO: Improve this by calculating, instead of hard-coding.

        let mut strategy = BasicStrategy {
            dealer_up_card: 0,
            hard_charts: [[(Decision::PlaceHolder, Decision::PlaceHolder); 10]; 14],
            soft_charts: [[(Decision::PlaceHolder, Decision::PlaceHolder); 10]; 9],
            pair_charts: [[(Decision::PlaceHolder, Decision::PlaceHolder); 10]; 10],
        };

        const H: (Decision, Decision) = (Decision::Hit, Decision::PlaceHolder);
        const S: (Decision, Decision) = (Decision::Stand, Decision::PlaceHolder);
        const P: (Decision, Decision) = (Decision::Split, Decision::PlaceHolder);
        const DH: (Decision, Decision) = (Decision::Double, Decision::Hit);
        const DS: (Decision, Decision) = (Decision::Double, Decision::Stand);
        const DP: (Decision, Decision) = (Decision::Double, Decision::Split);
        const RH: (Decision, Decision) = (Decision::Surrender, Decision::Hit);
        const RS: (Decision, Decision) = (Decision::Surrender, Decision::Stand);
        const RP: (Decision, Decision) = (Decision::Surrender, Decision::Split);

        strategy.hard_charts = [
            [H, H, H, H, H, H, H, H, H, H], // 5
            [H, H, H, H, H, H, H, H, H, H],
            [H, H, H, H, H, H, H, H, H, H],
            [H, H, H, H, H, H, H, H, H, H],
            [H, H, DH, DH, DH, DH, H, H, H, H],
            [H, DH, DH, DH, DH, DH, DH, DH, DH, H],
            [DH, DH, DH, DH, DH, DH, DH, DH, DH, DH],
            [H, H, H, S, S, S, H, H, H, H],
            [H, S, S, S, S, S, H, H, H, H],
            [H, S, S, S, S, S, H, H, H, H],
            [RH, S, S, S, S, S, H, H, H, RH],
            [RH, S, S, S, S, S, H, H, RH, RH],
            [RS, S, S, S, S, S, S, S, S, S], // 17
            [S, S, S, S, S, S, S, S, S, S],  // 18, 18+
        ];
        strategy.soft_charts = [
            [H, H, H, H, DH, DH, H, H, H, H], // Ace + 2
            [H, H, H, H, DH, DH, H, H, H, H],
            [H, H, H, DH, DH, DH, H, H, H, H],
            [H, H, H, DH, DH, DH, H, H, H, H],
            [H, H, DH, DH, DH, DH, H, H, H, H],
            [H, DS, DS, DS, DS, DS, S, S, H, H],
            [S, S, S, S, S, DS, S, S, S, S],
            [S, S, S, S, S, S, S, S, S, S], // Ace + 9
            [S, S, S, S, S, S, S, S, S, S], // Ace + 10
        ];
        strategy.pair_charts = [
            [P, P, P, P, P, P, P, P, P, P], // Double Ace
            [H, P, P, P, P, P, P, H, H, H], // Double 2
            [H, P, P, P, P, P, P, H, H, H],
            [H, H, H, H, P, P, H, H, H, H],
            [H, DH, DH, DH, DH, DH, DH, DH, DH, H],
            [H, P, P, P, P, P, H, H, H, H],
            [H, P, P, P, P, P, P, H, H, H],
            [RP, P, P, P, P, P, P, P, P, P],
            [S, P, P, P, P, P, S, P, P, S],
            [S, S, S, S, S, S, S, S, S, S], // Double 10
        ];

        strategy
    }

    fn init_with_initial_situation(&mut self, rule: &Rule, initial_situation: &InitialSituation) {
        self.dealer_up_card = initial_situation.dealer_up_card;
    }

    fn make_decision(
        &mut self,
        rule: &Rule,
        current_hand: &CardCount,
        current_split_all_times: u8,
        current_split_ace_times: u8,
    ) -> Decision {
        let col = (self.dealer_up_card - 1) as usize;

        let decision = {
            if current_hand.get_total() == 2
                && current_hand[(current_hand.get_sum() / 2) as u8] == 2
            {
                // Pair
                let row = (current_hand.get_sum() / 2) as usize;
                self.pair_charts[row][col]
            } else if current_hand.is_soft() && current_hand.get_sum() + 10 <= 21 {
                // Soft hand
                if current_hand[10] == 1 {
                    (Decision::Stand, Decision::PlaceHolder)
                } else {
                    let another_card = current_hand.get_sum() - 1;
                    let row = (another_card - 2) as usize;
                    self.soft_charts[row][col]
                }
            } else {
                // Hard hand
                let row = {
                    if current_hand.get_sum() <= 5 {
                        0
                    } else if current_hand.get_sum() >= 18 {
                        13
                    } else {
                        current_hand.get_sum() - 5
                    }
                } as usize;
                self.hard_charts[row][col]
            }
        };

        match decision.0 {
            Decision::Double => {
                if current_split_all_times == 0 || rule.allow_das {
                    Decision::Double
                } else {
                    decision.1
                }
            }
            Decision::Surrender => {
                if rule.allow_late_surrender {
                    Decision::Surrender
                } else {
                    decision.1
                }
            }
            _ => decision.0,
        }
    }
}

struct DpStrategy {
    sol: SolutionForInitialSituation,
    rule: Rule,
}

impl Strategy for DpStrategy {
    fn new(rule: &Rule) -> Self {
        DpStrategy {
            sol: Default::default(),
            rule: *rule,
        }
    }

    fn init_with_initial_situation(&mut self, rule: &Rule, initial_situation: &InitialSituation) {
        self.sol = calculate_solution_with_initial_situation(1, &self.rule, initial_situation);
    }

    fn make_decision(
        &mut self,
        rule: &Rule,
        current_hand: &CardCount,
        current_split_all_times: u8,
        current_split_ace_times: u8,
    ) -> Decision {
        if current_hand.get_total() == 2 && current_hand[1] == 2 {
            return Decision::Split;
        }

        let (_, decision) = get_max_expectation(&self.sol.ex_stand_hit, current_hand, rule);
        decision
    }
}
