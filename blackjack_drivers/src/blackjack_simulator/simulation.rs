use self::private::Statistics;
use blackjack::strategy::Strategy;
use blackjack_drivers::ConfigBlackjackSimulator;

mod private {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Statistics {
        current_money: i32,
        total_bet: u32,

        last_money: i32,
        last_bet: u32,

        min_money: i32,
    }

    impl Statistics {
        pub fn bet_money(&mut self, money: u32) {
            self.total_bet += money;
            self.current_money -= money as i32;
            if self.min_money > self.current_money {
                self.min_money = self.current_money;
            }
        }

        pub fn receive_money(&mut self, money: u32) {
            self.current_money += money as i32;
        }

        pub fn get_current_money(&self) -> i32 {
            self.current_money
        }

        pub fn get_total_bet(&self) -> u32 {
            self.total_bet
        }

        pub fn get_rate(&self) -> f64 {
            self.current_money as f64 / self.total_bet as f64
        }

        pub fn get_delta_money(&mut self) -> i32 {
            let ret = self.current_money - self.last_money;
            self.last_money = self.current_money;
            ret
        }

        pub fn get_delta_bet(&mut self) -> u32 {
            let ret = self.total_bet - self.last_bet;
            self.last_bet = self.total_bet;
            ret
        }

        pub fn get_min_money(&self) -> i32 {
            self.min_money
        }
    }
}

pub fn simulate_playing_forever(
    rule: &blackjack::Rule,
    simulator_config: &ConfigBlackjackSimulator,
) -> Result<(), String> {
    let mut dp_strategy =
        blackjack::strategy::DpStrategySinglePlayer::new(simulator_config.number_of_threads);
    let mut simulator = blackjack::simulation::Simulator::new(rule);

    // stat_virtual is used to do statistics when player places bets in each game.
    let mut stat_virtual: Statistics = Default::default();
    // stat_real is used to do statistics when player only places bets if expectation is positive.
    let mut stat_real: Statistics = Default::default();
    let mut prev_period_end_money = 0;
    let mut prev_period_end_bet = 0;
    let mut total_ex_sum = 0.0;

    let mut game_id: u64 = 0;
    const BASIC_BET: u32 = 100;

    loop {
        game_id += 1;
        println!("Game #{}", game_id);
        let shoe_card_count = simulator.get_shoe_card_count();
        println!("Number of cards in shoe: {}", shoe_card_count.get_total(),);
        const TOP_NUMBER_OF_CARDS: usize = 20;
        print!("Top {} cards:", TOP_NUMBER_OF_CARDS);
        for card in simulator.preview_next_few_cards_in_shoe(TOP_NUMBER_OF_CARDS) {
            print!(" {}", card);
        }
        println!();

        simulator.seat_player(1, 0)?;

        let total_ex =
            dp_strategy.calculate_expectation_before_bet(rule, simulator.get_shoe_card_count());
        total_ex_sum += total_ex;
        println!(
            "Expectation: {:.6}. Avg: {:.6}",
            total_ex,
            total_ex_sum / game_id as f64
        );
        let bet = {
            if total_ex <= 0.0 {
                0
            } else {
                BASIC_BET
            }
        };
        simulator.place_bets(BASIC_BET)?;
        stat_virtual.bet_money(BASIC_BET);
        stat_real.bet_money(bet);

        let initial_situation = simulator.deal_initial_cards()?;
        dp_strategy.init_with_initial_situation(rule, &initial_situation);

        let buy_insurance = dp_strategy.should_buy_insurance(rule, &initial_situation);
        if buy_insurance {
            println!("########## Should buy insurance! ###############");
            stat_virtual.bet_money(BASIC_BET / 2);
            stat_real.bet_money(bet / 2);
        }
        let dealer_does_peek_and_natural = simulator.dealer_peeks_if_necessary(buy_insurance)?;

        if !dealer_does_peek_and_natural {
            simulator.wait_for_right_players()?;
            simulator.stop_split()?;
            for group_id in 0..simulator.get_number_of_groups() {
                print!("Decisions for Group {}:", group_id);
                loop {
                    let hand_card_count = simulator.get_my_current_card_count();
                    let split_all_times = simulator.get_current_split_all_times();
                    let split_ace_times = simulator.get_current_split_ace_times();
                    let decision = dp_strategy.make_decision(
                        rule,
                        hand_card_count,
                        split_all_times,
                        split_ace_times,
                    );
                    print!(" {}", decision_to_string(decision));
                    if decision == blackjack::Decision::Double {
                        stat_virtual.bet_money(BASIC_BET);
                        stat_real.bet_money(bet);
                    }
                    let decision_fn = decision_to_fn(decision);
                    if decision_fn(&mut simulator)? {
                        break;
                    }
                }
                println!();
            }
            simulator.wait_for_left_players()?;
        }

        let winning_money = simulator.dealer_plays_and_summary()?;
        stat_virtual.receive_money(winning_money);
        stat_real.receive_money((winning_money as u64 * bet as u64 / BASIC_BET as u64) as u32);

        simulator.start_new_shoe_if_necessary()?;

        println!();
        print!("Virtual stat: ");
        println!(
            "Money: {}({}). Total bet: {}({}). Rate: {:.2}%. Min money: {}.",
            stat_virtual.get_current_money(),
            stat_virtual.get_delta_money(),
            stat_virtual.get_total_bet(),
            stat_virtual.get_delta_bet(),
            stat_virtual.get_rate() * 100.0,
            stat_virtual.get_min_money(),
        );
        print!("Real stat: ");
        println!(
            "Money: {}({}). Total bet: {}({}). Rate: {:.2}%. Min money: {}.",
            stat_real.get_current_money(),
            stat_real.get_delta_money(),
            stat_real.get_total_bet(),
            stat_real.get_delta_bet(),
            stat_real.get_rate() * 100.0,
            stat_real.get_min_money(),
        );

        println!();
        let mut period_percentage = game_id % simulator_config.games_in_period;
        let period_money = stat_real.get_current_money() - prev_period_end_money;
        let period_bet = stat_real.get_total_bet() - prev_period_end_bet;
        if period_percentage == 0 {
            period_percentage = simulator_config.games_in_period as u64;
            prev_period_end_money = stat_real.get_current_money();
            prev_period_end_bet = stat_real.get_total_bet();
        }
        let period_percentage =
            (period_percentage * 100 as u64) / simulator_config.games_in_period as u64;
        print!("This period: ");
        println!(
            "Money: {}, Total bet: {}, Rate: {:.2}%. Process: {:.2}%",
            period_money,
            period_bet,
            (period_money * 100) as f64 / period_bet as f64,
            period_percentage,
        );
        println!("---------------------------------------------------------------------");
    }
}

fn decision_to_fn(
    decision: blackjack::Decision,
) -> fn(&mut blackjack::simulation::Simulator) -> Result<bool, String> {
    match decision {
        blackjack::Decision::Stand => blackjack::simulation::Simulator::play_stand,
        blackjack::Decision::Hit => blackjack::simulation::Simulator::play_hit,
        blackjack::Decision::Double => blackjack::simulation::Simulator::play_double,
        blackjack::Decision::Surrender => blackjack::simulation::Simulator::play_surrender,
        _ => panic!("Impossible decision"),
    }
}

fn decision_to_string(decision: blackjack::Decision) -> String {
    match decision {
        blackjack::Decision::Stand => String::from("Stand"),
        blackjack::Decision::Hit => String::from("Hit"),
        blackjack::Decision::Double => String::from("~~~~~~DOUBLE~~~~~~"),
        blackjack::Decision::Surrender => String::from("Surrender"),
        _ => panic!("Impossible decision"),
    }
}
