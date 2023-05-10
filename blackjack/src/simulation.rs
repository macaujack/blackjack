pub mod hand;
pub mod shoe;

use crate::{
    strategy::Strategy, CardCount, Decision, HandState, InitialSituation, PeekPolicy, Rule,
};
use blackjack_macros::allowed_phase;
use strum_macros::EnumIter;

use self::{hand::Hand, shoe::Shoe};

static FACE_VALUE_TO_BLACKJACK_VALUE: [u8; 13] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 10, 10];
const MAX_PLAYER: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter)]
pub enum Suit {
    Diamond = 0,
    Club,
    Heart,
    Spade,
}

/// Represents a card in the real world with a suit and a face value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Card {
    pub face_value: u8,
    pub suit: Suit,
}

impl Card {
    pub fn blackjack_value(&self) -> u8 {
        FACE_VALUE_TO_BLACKJACK_VALUE[(self.face_value - 1) as usize]
    }
}

impl Default for Card {
    fn default() -> Self {
        Card {
            face_value: 1,
            suit: Suit::Diamond,
        }
    }
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let suit = match self.suit {
            Suit::Diamond => 'D',
            Suit::Club => 'C',
            Suit::Heart => 'H',
            Suit::Spade => 'S',
        };
        let value = match self.face_value {
            1 => 'A',
            2 => '2',
            3 => '3',
            4 => '4',
            5 => '5',
            6 => '6',
            7 => '7',
            8 => '8',
            9 => '9',
            10 => 'T',
            11 => 'J',
            12 => 'Q',
            13 => 'K',
            _ => panic!("Invalid card face value!"),
        };
        write!(f, "{}{}", suit, value)
    }
}

impl Into<u8> for Card {
    fn into(self) -> u8 {
        self.suit as u8 * 13 + self.face_value - 1
    }
}

impl TryFrom<u8> for Card {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= 52 {
            Err(())
        } else {
            let suit = match value / 13 {
                0 => Suit::Diamond,
                1 => Suit::Club,
                2 => Suit::Heart,
                3 => Suit::Spade,
                _ => panic!("Impossible to happen!"),
            };
            let card = Card {
                suit,
                face_value: value % 13 + 1,
            };
            Ok(card)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GamePhase {
    WaitForPlayerSeat,
    PlaceBets,
    DealInitialCards,
    DealerPeek,
    WaitForRightPlayers,
    PlaySplit,
    Play,
    WaitForLeftPlayers,
    DealerPlayAndSummary,
    StartNewShoe,
}

/// Simulates a Blackjack table. Note that there are some differences:
/// 1. Even when you place no bet, you can still play.
pub struct Simulator {
    rule: Rule,
    number_of_players: u8,
    seat_order: u8,

    // Game state
    current_game_phase: GamePhase,
    shoe: shoe::Shoe,
    dealer_hand: hand::Hand,
    insurance_bet: u32,

    // My playing state
    current_split_all_times: u8,
    current_split_ace_times: u8,
    current_playing_group_index: usize,
    current_hand: hand::Hand,
}

impl Simulator {
    pub fn new(rule: &Rule) -> Self {
        let mut shoe = shoe::Shoe::new(rule.number_of_decks, rule.cut_card_proportion);
        shoe.shuffle(0);
        Self {
            rule: *rule,
            number_of_players: 0,
            seat_order: 0,
            current_game_phase: GamePhase::WaitForPlayerSeat,
            shoe,
            dealer_hand: hand::Hand::new(),
            insurance_bet: 0,
            current_split_all_times: 0,
            current_split_ace_times: 0,
            current_playing_group_index: 0,
            current_hand: hand::Hand::new(),
        }
    }

    /// This will seat the player. Can be called at WaitForPlayerSeat phase.
    /// Call this with two zeros to indicate not changing.
    #[allowed_phase(WaitForPlayerSeat)]
    pub fn seat_player(&mut self, number_of_players: u8, seat_order: u8) -> Result<(), String> {
        if number_of_players > MAX_PLAYER {
            return Err(format!("number_of_players cannot exceed {}", MAX_PLAYER));
        }
        if seat_order >= number_of_players {
            return Err(format!("seat_order should be less than number_of_players"));
        }

        self.current_game_phase = GamePhase::PlaceBets;
        self.new_game();

        if number_of_players == 0 && seat_order == 0 {
            return Ok(());
        }
        self.number_of_players = number_of_players;
        self.seat_order = seat_order;
        Ok(())
    }

    /// This will perform automatic simulation according to given strategy and event handler.
    /// Can be called at PlaceBets phase.
    #[allowed_phase(PlaceBets)]
    pub fn automatic_simulate_with_fixed_main_bet<T: Strategy, U: SimulatorEventHandler>(
        &mut self,
        main_bet: u32,
        strategy: &mut T,
        handler: &mut U,
    ) -> Result<(), String> {
        handler.on_game_begin(&self.shoe);
        let ex_before_bet =
            strategy.calculate_expectation_before_bet(&self.rule, self.get_shoe_card_count());

        self.place_bets(main_bet)?;
        handler.on_bet_money(main_bet, ex_before_bet);

        let initial_situation = self.deal_initial_cards()?;
        strategy.init_with_initial_situation(&self.rule, &initial_situation);
        handler.on_deal_cards(&initial_situation);

        let dealer_cards = self.dealer_hand.get_cards(0);
        let dealer_peeks_and_gets_natural = {
            let up = dealer_cards[0].blackjack_value();
            let dealer_will_peek = match self.rule.peek_policy {
                PeekPolicy::UpAceOrTen => up == 1 || up == 10,
                PeekPolicy::UpAce => up == 1,
                PeekPolicy::NoPeek => false,
            };
            if dealer_will_peek {
                let buy_insurance = strategy.should_buy_insurance(&self.rule, &initial_situation);
                let insurance_bet = {
                    if buy_insurance {
                        main_bet / 2
                    } else {
                        0
                    }
                };
                handler.on_buy_insurance(insurance_bet);
                let dealer_gets_natural = self.dealer_peeks(buy_insurance)?;
                dealer_gets_natural
            } else {
                self.current_game_phase = GamePhase::WaitForRightPlayers;
                false
            }
        };

        if !dealer_peeks_and_gets_natural {
            self.wait_for_right_players()?;

            let split_time_limit = {
                let current_card = self.current_hand.get_cards(0);
                if current_card[0].blackjack_value() == current_card[1].blackjack_value() {
                    if current_card[0].blackjack_value() == 1 {
                        self.rule.split_ace_limits
                    } else {
                        self.rule.split_all_limits
                    }
                } else {
                    0
                }
            };
            for _ in 0..split_time_limit {
                let hand_groups = self.get_all_card_counts();
                let group_index = strategy.should_split(
                    &self.rule,
                    self.get_shoe_card_count(),
                    initial_situation.dealer_up_card,
                    &hand_groups,
                );

                if group_index.is_none() {
                    break;
                }
                let group_index = group_index.unwrap();
                self.play_split(group_index)?;
                handler.on_split(&self.current_hand);
            }
            self.stop_split()?;

            if self.current_split_ace_times == 0 {
                if self.current_split_all_times == 0 {
                    self.loop_make_decisions_single(strategy, handler);
                } else {
                    self.loop_make_decisions_multiple(strategy, handler);
                }
            }

            self.wait_for_left_players()?;
        } else {
            handler.on_game_early_end();
        }

        let returned_money = self.dealer_plays_and_summary()?;
        handler.on_summary_game(&self.current_hand, &self.dealer_hand, returned_money);

        self.start_new_shoe_if_necessary()?;

        Ok(())
    }

    /// Can be called at PlaceBets phase.
    /// Place 0 bet to indicate not to place any bet this time.
    #[allowed_phase(PlaceBets)]
    pub fn place_bets(&mut self, bet: u32) -> Result<(), String> {
        if (bet as f64 * self.rule.payout_blackjack).fract() != 0.0 {
            return Err(format!(
                "bet multiplied by payout_blackjack must be an integer"
            ));
        }
        if bet % 2 != 0 {
            return Err(format!(
                "bet must be an even integer to possibly buy insurance"
            ));
        }
        if ((bet / 2) as f64 * self.rule.payout_insurance).fract() != 0.0 {
            return Err(format!(
                "Half of bet multiplied by payout_insurance must be an integer"
            ));
        }
        self.current_hand.set_original_bet(bet);
        self.current_game_phase = GamePhase::DealInitialCards;
        Ok(())
    }

    /// Can be called at DealInitialCards phase.
    /// Call this to deal initial cards to each player and dealer herself.
    /// Returns InitialSituation.
    #[allowed_phase(DealInitialCards)]
    pub fn deal_initial_cards(&mut self) -> Result<InitialSituation, String> {
        for _ in 0..2 {
            for i in 0..self.number_of_players {
                let card = self.shoe.deal_card().unwrap();
                if i == self.seat_order {
                    self.receive_card_for_me(card);
                }
            }
            let card = self.shoe.deal_card().unwrap();
            self.receive_card_for_dealer(card);
        }

        self.current_game_phase = GamePhase::DealerPeek;
        let hand_cards = self.current_hand.get_cards(0);
        let dealer_up_card = self.dealer_hand.get_cards(0)[0];

        let initial_situation = InitialSituation::new(
            self.get_shoe_card_count().clone(),
            (
                hand_cards[0].blackjack_value(),
                hand_cards[1].blackjack_value(),
            ),
            dealer_up_card.blackjack_value(),
        );
        Ok(initial_situation)
    }

    /// Can be called at DealerPeek phase.
    /// Call this to make dealer peeks her hole card if necessary.
    /// Returns true if dealer does peek and gets a natural. Otherwise false.
    #[allowed_phase(DealerPeek)]
    pub fn dealer_peeks_if_necessary(&mut self, buy_insurance: bool) -> Result<bool, String> {
        let dealer_cards = self.dealer_hand.get_cards(0);
        let up = dealer_cards[0].blackjack_value();
        let dealer_will_peek = match self.rule.peek_policy {
            PeekPolicy::UpAceOrTen => up == 1 || up == 10,
            PeekPolicy::UpAce => up == 1,
            PeekPolicy::NoPeek => false,
        };
        if !dealer_will_peek {
            if buy_insurance {
                return Err(format!("Cannot buy insurance when dealer doesn't peek!"));
            }
            self.current_game_phase = GamePhase::WaitForRightPlayers;
            return Ok(false);
        }

        self.dealer_peeks(buy_insurance)
    }

    fn dealer_peeks(&mut self, buy_insurance: bool) -> Result<bool, String> {
        if buy_insurance {
            self.insurance_bet = self.current_hand.get_bet(0) / 2;
        }

        let dealer_cards = self.dealer_hand.get_cards(0);
        let up = dealer_cards[0].blackjack_value();
        let hole = dealer_cards[1].blackjack_value();
        let dealer_is_natural = up + hole == 11;
        if dealer_is_natural {
            self.insurance_bet += ((self.insurance_bet as f64) * self.rule.payout_insurance) as u32;
            self.current_game_phase = GamePhase::DealerPlayAndSummary;
        } else {
            self.insurance_bet = 0;
            self.current_game_phase = GamePhase::WaitForRightPlayers;
        }
        Ok(dealer_is_natural)
    }

    /// Can be called at WaitForRightPlayers phase.
    /// Call this to wait for players on your right.
    #[allowed_phase(WaitForRightPlayers)]
    pub fn wait_for_right_players(&mut self) -> Result<(), String> {
        // Simply let them stand immediately.
        self.current_game_phase = GamePhase::PlaySplit;
        Ok(())
    }

    /// Can be called at PlaySplit phase.
    /// Call this to play hand.
    /// Returns true if you reach split times limit and cannot make more splits.
    ///
    /// Note that if you are splitting Aces, you cannot make other decisions.
    #[allowed_phase(PlaySplit)]
    pub fn play_split(&mut self, group_index: usize) -> Result<bool, String> {
        if self.reached_split_time_limits() {
            return Err(format!("You reached split time limits!"));
        }
        let cards = self.current_hand.get_cards(group_index);
        if cards[0].blackjack_value() != cards[1].blackjack_value() {
            return Err(format!("You cannot split two cards with different values!"));
        }

        self.current_split_all_times += 1;
        if cards[0].blackjack_value() == 1 {
            self.current_split_ace_times += 1;
        }

        self.current_hand.split_group(group_index);
        let card = self.shoe.deal_card().unwrap();
        self.current_hand.receive_card(group_index, card);
        let card = self.shoe.deal_card().unwrap();
        self.current_hand
            .receive_card(self.current_hand.get_number_of_groups() - 1, card);

        Ok(self.reached_split_time_limits())
    }

    /// Can be called at PlaySplit phase.
    /// Call this stop Split and proceed to the next game phase.
    ///
    /// Note that if you just splitted Aces, you won't be able to make other decisions,
    /// so the Play phase will be skipped.
    #[allowed_phase(PlaySplit)]
    pub fn stop_split(&mut self) -> Result<(), String> {
        self.current_game_phase = {
            if self.current_split_ace_times > 0 {
                GamePhase::WaitForLeftPlayers
            } else {
                GamePhase::Play
            }
        };
        Ok(())
    }

    /// Can be called at Play phase.
    /// Returns true if cannot play current hand group any more.
    #[allowed_phase(Play)]
    pub fn play_stand(&mut self) -> Result<bool, String> {
        self.move_to_next_group();
        Ok(true)
    }

    /// Can be called at Play phase.
    /// Returns true if cannot play current hand group any more.
    #[allowed_phase(Play)]
    pub fn play_hit(&mut self) -> Result<bool, String> {
        let card = self.shoe.deal_card().unwrap();
        self.receive_card_for_me(card);
        let my_card_count = self.get_my_current_card_count();
        if my_card_count.bust() {
            self.determine_winning(0.0);
            self.move_to_next_group();
            return Ok(true);
        }
        if my_card_count.get_total() == self.rule.charlie_number as u16 {
            self.determine_winning(2.0);
            self.move_to_next_group();
            return Ok(true);
        }

        Ok(false)
    }

    /// Can be called at Play phase.
    /// Returns true if cannot play current hand group any more.
    #[allowed_phase(Play)]
    pub fn play_double(&mut self) -> Result<bool, String> {
        let my_card_count = self.get_my_current_card_count();
        if my_card_count.get_total() != 2 {
            return Err(format!("You can only double down on initial 2 cards"));
        }
        if self.current_hand.get_number_of_groups() > 1 && !self.rule.allow_das {
            return Err(format!("DAS is not allowed"));
        }

        let card = self.shoe.deal_card().unwrap();
        self.receive_card_for_me(card);
        self.current_hand
            .double_down(self.current_playing_group_index);
        let my_card_count = self.get_my_current_card_count();
        if my_card_count.bust() {
            self.determine_winning(0.0);
        }
        self.move_to_next_group();
        Ok(true)
    }

    /// Can be called at Play phase.
    /// Returns true if cannot play current hand group any more.
    #[allowed_phase(Play)]
    pub fn play_surrender(&mut self) -> Result<bool, String> {
        if !self.rule.allow_late_surrender {
            return Err(format!("Surrender is not allowed!"));
        }
        self.determine_winning(0.5);
        self.move_to_next_group();
        Ok(true)
    }

    /// Can be called at WaitForLeftPlayers phase.
    /// Call this to wait for players on your left.
    #[allowed_phase(WaitForLeftPlayers)]
    pub fn wait_for_left_players(&mut self) -> Result<(), String> {
        // Simply let them stand immediately.
        self.current_game_phase = GamePhase::DealerPlayAndSummary;
        Ok(())
    }

    /// Can be called at DealerPlayAndSummary phase.
    /// Call this to make dealer play according to game rule.
    /// Returns the total money you win including all side bets.
    /// Note that this is what you win, not your profit. For example,
    /// you wager 10 dollars. If you win, you win 20. If you lose,
    /// you win 0.
    #[allowed_phase(DealerPlayAndSummary)]
    pub fn dealer_plays_and_summary(&mut self) -> Result<u32, String> {
        let main_win = loop {
            let dealer_card_count = self.get_dealer_card_count();
            let must_stand = {
                let actual_sum = dealer_card_count.get_actual_sum();
                let is_soft = dealer_card_count.is_soft();
                if actual_sum > 17 {
                    true
                } else if actual_sum < 17 {
                    false
                } else {
                    if !is_soft {
                        true
                    } else {
                        !self.rule.dealer_hit_on_soft17
                    }
                }
            };

            if must_stand {
                let mut total_win = 0;
                for i in 0..self.current_hand.get_number_of_groups() {
                    let my_card_count = self.current_hand.get_card_counts(i);
                    let mut this_group_win = self.current_hand.get_bet(i);

                    if self.current_hand.is_winning_already_determined(i) {
                        this_group_win = self.current_hand.get_bet(i);
                    } else if my_card_count.is_natural()
                        && self.current_hand.get_number_of_groups() == 1
                    {
                        if !dealer_card_count.is_natural() {
                            this_group_win +=
                                (this_group_win as f64 * self.rule.payout_blackjack) as u32;
                        }
                    } else if dealer_card_count.bust() {
                        this_group_win *= 2;
                    } else if dealer_card_count.is_natural() {
                        this_group_win = 0;
                    } else if my_card_count.get_actual_sum() < dealer_card_count.get_actual_sum() {
                        this_group_win = 0;
                    } else if my_card_count.get_actual_sum() > dealer_card_count.get_actual_sum() {
                        this_group_win *= 2;
                    }
                    total_win += this_group_win;
                }

                break total_win;
            }

            let card = self.shoe.deal_card().unwrap();
            self.receive_card_for_dealer(card);
        };

        self.current_game_phase = GamePhase::StartNewShoe;
        let insurance_win = self.insurance_bet;
        Ok(main_win + insurance_win)
    }

    /// Can be called at StartNewShoe phase.
    /// Call this to use a new shoe for playing if cut card is reached.
    #[allowed_phase(StartNewShoe)]
    pub fn start_new_shoe_if_necessary(&mut self) -> Result<(), String> {
        if self.shoe.reached_cut_card() {
            self.shoe.shuffle(0);
        }
        self.current_game_phase = GamePhase::WaitForPlayerSeat;
        Ok(())
    }

    pub fn reached_split_time_limits(&self) -> bool {
        self.current_split_all_times == self.rule.split_all_limits
            || self.current_split_ace_times == self.rule.split_ace_limits
    }

    pub fn get_shoe_card_count(&self) -> &CardCount {
        &self.shoe.get_card_count()
    }

    pub fn get_current_split_all_times(&self) -> u8 {
        self.current_split_all_times
    }

    pub fn get_current_split_ace_times(&self) -> u8 {
        self.current_split_ace_times
    }

    pub fn get_number_of_groups(&self) -> usize {
        self.current_hand.get_number_of_groups()
    }

    pub fn get_my_current_card_count(&self) -> &CardCount {
        self.current_hand
            .get_card_counts(self.current_playing_group_index)
    }

    pub fn get_dealer_card_count(&self) -> &CardCount {
        self.dealer_hand.get_card_counts(0)
    }

    pub fn preview_next_few_cards_in_shoe(&self, number: usize) -> &[Card] {
        self.shoe.preview_next_few_cards(number)
    }

    pub fn get_all_card_counts(&self) -> Vec<&CardCount> {
        let mut hand_groups: Vec<&CardCount> = Vec::with_capacity(self.get_number_of_groups());
        for i in 0..self.get_number_of_groups() {
            let hand_group = self.current_hand.get_card_counts(i);
            hand_groups.push(hand_group);
        }
        hand_groups
    }

    fn receive_card_for_me(&mut self, card: Card) {
        self.current_hand
            .receive_card(self.current_playing_group_index, card);
    }

    fn receive_card_for_dealer(&mut self, card: Card) {
        self.dealer_hand.receive_card(0, card);
    }

    fn determine_winning(&mut self, multiplier: f64) {
        self.current_hand
            .determine_winning(self.current_playing_group_index, multiplier);
    }

    /// Move current playing group to the next group. If no more group, the game phase will proceed.
    fn move_to_next_group(&mut self) {
        self.current_playing_group_index += 1;
        if self.current_playing_group_index == self.current_hand.get_number_of_groups() {
            self.current_game_phase = GamePhase::WaitForLeftPlayers;
        }
    }

    fn new_game(&mut self) {
        self.dealer_hand.clear();
        self.current_split_all_times = 0;
        self.current_split_ace_times = 0;
        self.current_playing_group_index = 0;
        self.current_hand.clear();
        self.insurance_bet = 0;
    }

    fn loop_make_decisions_single<T: Strategy, U: SimulatorEventHandler>(
        &mut self,
        strategy: &mut T,
        handler: &mut U,
    ) {
        loop {
            let current_hand = self.get_my_current_card_count();
            let decision = strategy.make_decision_single(&self.rule, current_hand);
            handler.on_make_decision(decision, 0);
            let finished = match decision {
                Decision::Stand => self.play_stand().unwrap(),
                Decision::Hit => {
                    let finished = self.play_hit().unwrap();
                    self.check_bust_or_charlie(handler, 0);
                    finished
                }
                Decision::Double => {
                    let finished = self.play_double().unwrap();
                    self.check_bust_or_charlie(handler, 0);
                    finished
                }
                Decision::Surrender => self.play_surrender().unwrap(),
                _ => panic!("Invalid decision!"),
            };

            if finished {
                break;
            }
        }
    }

    fn loop_make_decisions_multiple<T: Strategy, U: SimulatorEventHandler>(
        &mut self,
        strategy: &mut T,
        handler: &mut U,
    ) {
        let mut hand_states: Vec<HandState> = Vec::with_capacity(self.get_number_of_groups() - 1);
        for group_index in 0..self.get_number_of_groups() {
            loop {
                let current_hands = self.get_all_card_counts();
                let decision =
                    strategy.make_decision_multiple(&self.rule, &current_hands, &hand_states);
                handler.on_make_decision(decision, group_index);
                let finished = match decision {
                    Decision::Stand => {
                        self.play_stand().unwrap();
                        hand_states.push(HandState::Normal);
                        true
                    }
                    Decision::Hit => {
                        let finished = self.play_hit().unwrap();
                        if finished {
                            hand_states.push(HandState::Normal);
                            self.check_bust_or_charlie(handler, group_index);
                        }
                        finished
                    }
                    Decision::Double => {
                        self.play_double().unwrap();
                        hand_states.push(HandState::Double);
                        self.check_bust_or_charlie(handler, group_index);
                        true
                    }
                    Decision::Surrender => {
                        self.play_surrender().unwrap();
                        hand_states.push(HandState::Surrender);
                        true
                    }
                    _ => panic!("Invalid decision!"),
                };

                if finished {
                    break;
                }
            }
        }
    }

    fn check_bust_or_charlie<U: SimulatorEventHandler>(&self, handler: &mut U, group_index: usize) {
        let current_hand = self.current_hand.get_card_counts(group_index);
        if current_hand.bust() {
            handler.on_player_bust(group_index);
        } else if current_hand.get_total() == self.rule.charlie_number as u16 {
            handler.on_player_charlie(group_index);
        }
    }
}

pub trait SimulatorEventHandler {
    fn on_game_begin(&mut self, shoe: &Shoe);
    fn on_bet_money(&mut self, bet: u32, ex_before_bet: f64);
    fn on_deal_cards(&mut self, initial_situation: &InitialSituation);
    fn on_buy_insurance(&mut self, insurance_bet: u32);
    fn on_game_early_end(&mut self);
    fn on_split(&mut self, player_hand: &Hand);
    fn on_make_decision(&mut self, decision: Decision, group_index: usize);
    fn on_player_bust(&mut self, group_index: usize);
    fn on_player_charlie(&mut self, group_index: usize);
    fn on_summary_game(&mut self, player_hand: &Hand, dealer_hand: &Hand, returned_money: u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_typical_rule() -> Rule {
        Rule {
            number_of_decks: 8,
            cut_card_proportion: 0.5,
            split_all_limits: 1,
            split_ace_limits: 1,
            double_policy: crate::DoublePolicy::AnyTwo,
            dealer_hit_on_soft17: false,
            allow_das: false,
            allow_late_surrender: false,
            peek_policy: crate::PeekPolicy::UpAce,
            charlie_number: 6,

            payout_blackjack: 1.5,
            payout_insurance: 2.0,
        }
    }

    #[test]
    fn test_allowed_phase() {
        let rule = get_typical_rule();
        let mut simulator = Simulator::new(&rule);
        assert_eq!(simulator.current_game_phase, GamePhase::WaitForPlayerSeat);
        assert!(simulator.seat_player(1, 0).is_ok());
        assert_eq!(simulator.current_game_phase, GamePhase::PlaceBets);
        assert!(simulator.seat_player(0, 0).is_err());
    }
}
