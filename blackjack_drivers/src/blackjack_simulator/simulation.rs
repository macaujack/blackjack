use std::println;

use self::private::Statistics;
use blackjack::simulation::{Card, SimulatorEventHandler};
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

#[derive(Debug, Clone, Default)]
struct Handler {
    game_id: u64,
    number_of_cards_in_shoe_before_game: u16,
    top_cards_before_game: Vec<Card>,
    ex_before_bet: f64,
    sum_ex_before_bet: f64,
    buy_insurance: bool,
    should_split: bool,
    decisions: Vec<Vec<String>>,

    stat_virtual: Statistics,
    stat_real: Statistics,
}

impl SimulatorEventHandler for Handler {
    fn on_game_begin(&mut self, shoe: &blackjack::simulation::shoe::Shoe) {
        self.game_id += 1;
        self.number_of_cards_in_shoe_before_game = shoe.get_card_count().get_total();
        self.top_cards_before_game = shoe.preview_next_few_cards(20).to_vec();
        self.buy_insurance = false;
        self.should_split = false;
        self.decisions.clear();
    }

    fn on_calculate_expectation(&mut self, expectation: f64) {
        // TODO: Delete Debug
        println!("################################################");
        println!("Expectation is {}", expectation);
        println!("################################################");
        self.ex_before_bet = expectation;
        self.sum_ex_before_bet += expectation;
    }

    fn on_bet_money(&mut self, bet: u32) {
        self.stat_virtual.bet_money(bet);
        if self.ex_before_bet > 0.0 {
            self.stat_real.bet_money(bet);
        }
    }

    fn on_deal_cards(&mut self, _: &blackjack::InitialSituation) {}

    fn on_buy_insurance(&mut self, insurance_bet: u32) {
        self.buy_insurance = insurance_bet > 0;
        self.stat_virtual.bet_money(insurance_bet);
        if self.ex_before_bet > 0.0 {
            self.stat_real.bet_money(insurance_bet);
        }
    }

    fn on_game_early_end(&mut self) {}

    fn on_make_decision(&mut self, decision: blackjack::Decision, group_index: usize) {
        while self.decisions.len() <= group_index {
            self.decisions.push(Vec::new());
        }
        self.decisions[group_index].push(decision_to_string(decision));

        if decision == blackjack::Decision::Split {
            self.should_split = true;
        }
    }

    fn on_player_bust(&mut self, group_index: usize) {
        while self.decisions.len() <= group_index {
            self.decisions.push(Vec::new());
        }
        self.decisions[group_index].push(String::from("BUST"));
    }

    fn on_player_charlie(&mut self, group_index: usize) {
        while self.decisions.len() <= group_index {
            self.decisions.push(Vec::new());
        }
        self.decisions[group_index].push(String::from("CHARLIE"));
    }

    fn on_summary_game(
        &mut self,
        player_hand: &blackjack::simulation::hand::Hand,
        dealer_hand: &blackjack::simulation::hand::Hand,
        returned_money: u32,
    ) {
        println!("Game #{}", self.game_id);
        print!(
            "Top {} (of {}) cards: ",
            self.top_cards_before_game.len(),
            self.number_of_cards_in_shoe_before_game
        );
        for card in &self.top_cards_before_game {
            print!(" {}", card);
        }
        println!();

        println!(
            "Expectation: {:.6}   Avg: {:.6}",
            self.ex_before_bet,
            self.sum_ex_before_bet / self.game_id as f64
        );

        if self.buy_insurance {
            println!("############## Should buy insurance! ############");
        }
        if self.should_split {
            println!("$$$$$$$$$$$$$$ Should split! $$$$$$$$$$$$$");
        }
        println!();

        for (group_index, decisions) in self.decisions.iter().enumerate() {
            print!("Decisions for Group {}:", group_index);
            for decision in decisions {
                print!(" {}", decision);
            }
            println!();
        }
        println!();

        print!("Dealer cards:");
        for card in dealer_hand.get_cards(0) {
            print!(" {}", card);
        }
        println!();
        for group_index in 0..player_hand.get_number_of_groups() {
            print!("Player hand group {}:", group_index);
            let hand = player_hand.get_cards(group_index);
            for card in hand {
                print!(" {}", card);
            }
            println!();
        }
        println!();

        self.stat_virtual.receive_money(returned_money);
        if self.ex_before_bet > 0.0 {
            self.stat_real.receive_money(returned_money);
        }

        print!("Virtual stat: ");
        println!(
            "Money: {}({}). Total bet: {}({}). Rate: {:.2}%. Min money: {}.",
            self.stat_virtual.get_current_money(),
            self.stat_virtual.get_delta_money(),
            self.stat_virtual.get_total_bet(),
            self.stat_virtual.get_delta_bet(),
            self.stat_virtual.get_rate() * 100.0,
            self.stat_virtual.get_min_money(),
        );
        print!("Real stat: ");
        println!(
            "Money: {}({}). Total bet: {}({}). Rate: {:.2}%. Min money: {}.",
            self.stat_real.get_current_money(),
            self.stat_real.get_delta_money(),
            self.stat_real.get_total_bet(),
            self.stat_real.get_delta_bet(),
            self.stat_real.get_rate() * 100.0,
            self.stat_real.get_min_money(),
        );
        println!("----------------------------------------------------");
    }
}

pub fn simulate_playing_forever(
    rule: &blackjack::Rule,
    simulator_config: &ConfigBlackjackSimulator,
) -> Result<(), String> {
    let mut dp_strategy =
        blackjack::strategy::DpStrategySinglePlayer::new(simulator_config.number_of_threads);
    let mut handler: Handler = Default::default();
    let mut simulator = blackjack::simulation::Simulator::new(rule);

    loop {
        simulator.seat_player(1, 0)?;
        simulator.automatic_simulate_with_fixed_main_bet(100, &mut dp_strategy, &mut handler)?;
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
