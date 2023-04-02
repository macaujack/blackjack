#[cfg(test)]
mod tests {
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    use crate::{
        calculation::{calculate_solution, SolutionForInitialSituation},
        CardCount, Decision, InitialSituation, Rule, StateArray,
    };

    struct Shoe {
        number_of_decks: u8,
        cut_card_index: usize,
        cards: Vec<u8>,
        current_index: usize,
    }

    impl Shoe {
        fn new(number_of_decks: u8, cut_card_proportion: f64) -> Shoe {
            Shoe {
                number_of_decks,
                cut_card_index: (cut_card_proportion * (number_of_decks as u16 * 52) as f64)
                    as usize,
                cards: generate_random_shoe(number_of_decks),
                current_index: 0,
            }
        }

        fn reinit(&mut self) {
            self.cards.shuffle(&mut thread_rng());
            self.current_index = 0;
        }

        fn retry(&mut self) {
            self.current_index = 0;
        }

        fn deal_card(&mut self) -> (u8, bool) {
            self.current_index += 1;
            (
                self.cards[self.current_index - 1],
                self.current_index >= self.cut_card_index,
            )
        }

        fn reached_cut_card(&self) -> bool {
            self.current_index >= self.cut_card_index
        }
    }

    fn generate_random_shoe(number_of_decks: u8) -> Vec<u8> {
        let number_of_decks = number_of_decks as usize;
        let mut ret: Vec<u8> = vec![0; number_of_decks * 52];
        let mut idx = 0;
        for i in 1..10 {
            for _ in 0..number_of_decks * 4 {
                ret[idx] = i;
                idx += 1;
            }
        }
        for _ in 0..number_of_decks * 16 {
            ret[idx] = 10;
            idx += 1;
        }

        ret.shuffle(&mut thread_rng());
        ret
    }

    trait Strategy {
        fn init(&mut self, rule: &Rule, initial_situation: &InitialSituation);
        fn make_decision(&self, current_hand: &CardCount) -> Decision;
    }

    #[derive(Default)]
    struct BasicStrategy {
        dealer_up_card: u8,
        hard_charts: [[Decision; 10]; 14],
        soft_charts: [[Decision; 10]; 9],
    }

    const H: Decision = Decision::Hit;
    const S: Decision = Decision::Stand;
    const D: Decision = Decision::Double;
    const R: Decision = Decision::Surrender;

    impl Strategy for BasicStrategy {
        fn init(&mut self, rule: &Rule, initial_situation: &InitialSituation) {
            self.dealer_up_card = initial_situation.dealer_up_card;
            self.hard_charts = [
                [H, H, H, H, H, H, H, H, H, H], // 5
                [H, H, H, H, H, H, H, H, H, H],
                [H, H, H, H, H, H, H, H, H, H],
                [H, H, H, H, H, H, H, H, H, H],
                [H, H, D, D, D, D, H, H, H, H],
                [H, D, D, D, D, D, D, D, D, H],
                [D, D, D, D, D, D, D, D, D, D],
                [H, H, H, S, S, S, H, H, H, H],
                [H, S, S, S, S, S, H, H, H, H],
                [H, S, S, S, S, S, H, H, H, H],
                [R, S, S, S, S, S, H, H, H, R],
                [R, S, S, S, S, S, H, H, R, R],
                [R, S, S, S, S, S, S, S, S, S], // 17
                [S, S, S, S, S, S, S, S, S, S], // 18, 18+
            ];
            self.soft_charts = [
                [H, H, H, H, D, D, H, H, H, H], // Ace + 2
                [H, H, H, H, D, D, H, H, H, H],
                [H, H, H, D, D, D, H, H, H, H],
                [H, H, H, D, D, D, H, H, H, H],
                [H, H, D, D, D, D, H, H, H, H],
                [H, D, D, D, D, D, S, S, H, H],
                [S, S, S, S, S, D, S, S, S, S],
                [S, S, S, S, S, S, S, S, S, S], // Ace + 9
                [S, S, S, S, S, S, S, S, S, S], // Ace + 10
            ]
        }

        fn make_decision(&self, current_hand: &CardCount) -> Decision {
            let col = (self.dealer_up_card - 1) as usize;
            if current_hand.get_total() == 2 && current_hand[1] == 2 {
                return Decision::Split;
            }

            if current_hand.is_soft() && current_hand.get_sum() + 10 <= 21 {
                if current_hand[10] == 1 {
                    Decision::Stand
                } else {
                    let another_card = current_hand.get_sum() - 1;
                    let row = (another_card - 2) as usize;
                    self.soft_charts[row][col]
                }
            } else {
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
        }
    }

    struct MyStrategy {
        sol: SolutionForInitialSituation,
    }

    impl Strategy for MyStrategy {
        fn init(&mut self, rule: &Rule, initial_situation: &InitialSituation) {
            self.sol = calculate_solution(rule, initial_situation);
        }
        fn make_decision(&self, current_hand: &CardCount) -> Decision {
            if current_hand.get_total() == 2 && current_hand[1] == 2 {
                return Decision::Split;
            }

            let (_, decision) =
                self.sol.general_solution[current_hand].get_max_expectation(current_hand.bust());
            decision
        }
    }

    #[test]
    fn test_generate_random_shoe() {
        let shoe = generate_random_shoe(1);
        println!("{:#?}", shoe);
    }

    // Bet 100
    fn play_a_round<T: Strategy>(rule: &Rule, strategy: &mut T, shoe: &mut Shoe) -> (i32, bool) {
        let (my_first_card, _) = shoe.deal_card();
        let (dealer_up_card, _) = shoe.deal_card();
        let (my_second_card, _) = shoe.deal_card();
        let mut counts = [0; 10];
        for i in shoe.current_index..shoe.cards.len() {
            counts[(shoe.cards[i] - 1) as usize] += 1;
        }
        let initial_shoe = CardCount::new(&counts);

        let initial_situation = InitialSituation::new(
            initial_shoe,
            (my_first_card, my_second_card),
            dealer_up_card,
        );
        strategy.init(rule, &initial_situation);
        let (dealer_hole_card, _) = shoe.deal_card();

        let mut current_hand = CardCount::new(&[0; 10]);
        current_hand.add_card(my_first_card);
        current_hand.add_card(my_second_card);

        let dealer_natural_blackjack = dealer_up_card + dealer_hole_card == 11
            && (dealer_up_card == 1 || dealer_hole_card == 1);
        let me_natural_blackjack = current_hand.get_sum() == 11 && current_hand.is_soft();

        if dealer_natural_blackjack {
            if me_natural_blackjack {
                return (0, shoe.reached_cut_card());
            }
            return (-100, shoe.reached_cut_card());
        }

        let mut bet = 100;
        let mut has_surrendered = false;
        loop {
            if current_hand.get_sum() > 21 {
                break;
            }
            let my_decision = strategy.make_decision(&current_hand);
            print!("{:#?} ", my_decision);
            match my_decision {
                Decision::Hit => {
                    let (card, _) = shoe.deal_card();
                    current_hand.add_card(card);
                }
                Decision::Stand => {
                    break;
                }
                Decision::Double => {
                    let (card, _) = shoe.deal_card();
                    current_hand.add_card(card);
                    bet *= 2;
                    break;
                }
                Decision::Surrender => {
                    has_surrendered = true;
                    break;
                }
                _ => {
                    panic!("wtf??")
                }
            }
        }
        println!();

        let my_sum = {
            if current_hand.is_soft() && current_hand.get_sum() + 10 <= 21 {
                current_hand.get_sum() + 10
            } else {
                current_hand.get_sum()
            }
        };

        let mut dealer_sum = dealer_up_card + dealer_hole_card;
        let mut dealer_soft = dealer_up_card == 1 || dealer_hole_card == 1;
        while !(dealer_sum >= 17 || dealer_soft && dealer_sum + 10 > 17 && dealer_sum + 10 <= 21) {
            let (card, _) = shoe.deal_card();
            dealer_soft = dealer_soft || card == 1;
            dealer_sum += card;
        }
        if dealer_sum < 17 {
            dealer_sum += 10;
        }
        let dealer_sum = dealer_sum as u16;

        if has_surrendered {
            bet = -bet / 2;
        } else if my_sum > 21 {
            bet = -bet;
        } else if me_natural_blackjack {
            bet += bet / 2;
        } else if dealer_sum <= 21 {
            if my_sum < dealer_sum {
                bet = -bet;
            } else if my_sum == dealer_sum {
                bet = 0;
            }
        }

        (bet, shoe.reached_cut_card())
    }

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
    fn test_strategy_on_new_shoe() {
        let mut shoe = Shoe::new(8, 0.5);
        let mut basic_strategy: BasicStrategy = Default::default();
        let mut my_strategy: MyStrategy = MyStrategy {
            sol: SolutionForInitialSituation {
                general_solution: StateArray::new(),
                split_expectation: 0.0,
            },
        };
        let rule = get_typical_rule();

        let mut acc_basic: i32 = 0;
        let mut acc_my: i32 = 0;
        let total_rounds = 1000;
        for round in 0..total_rounds {
            shoe.reinit();
            while shoe.cards[0] == 1 && shoe.cards[2] == 1 {
                shoe.reinit();
            }
            // shoe.cards[0] = 1;
            // shoe.cards[1] = 9;
            // shoe.cards[2] = 2;
            // shoe.cards[3] = 10;
            // shoe.cards[4] = 8;
            // shoe.cards[5] = 10;
            // shoe.cards[6] = 7;
            // shoe.cards[7] = 2;
            // shoe.cards[8] = 9;
            // shoe.cards[9] = 2;
            // shoe.cards[10] = 10;
            print!("Turn #{}: ", round);
            for i in 0..20 {
                print!("{} ", shoe.cards[i]);
            }
            println!();
            let (profit_basic, _) = play_a_round(&rule, &mut basic_strategy, &mut shoe);
            acc_basic += profit_basic;
            shoe.retry();
            let (profit_my, _) = play_a_round(&rule, &mut my_strategy, &mut shoe);
            acc_my += profit_my;
            println!("Turn #{}: {:#?}, {:#?}", round, acc_basic, acc_my);
        }
        println!();
        println!("Acc: {}, {}", acc_basic, acc_my);
        println!("Total rounds: {}", total_rounds);
    }
}
