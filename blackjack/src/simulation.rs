pub mod hand;
pub mod shoe;
pub mod strategy;

use crate::Rule;
use blackjack_macros::allowed_phase;
use strum_macros::EnumIter;

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
    Play,
    WaitForLeftPlayers,
    StartNewShoe,
}

pub struct Simulator {
    rule: Rule,
    number_of_players: u8,
    seat_order: u8,

    // Game state
    current_game_phase: GamePhase,
    shoe: shoe::Shoe,
    dealer_up_card: Card,

    // My playing state
    current_split_all_times: u8,
    current_split_ace_times: u8,
    current_playing_group_index: usize,
    current_hand: hand::Hand,
}

impl Simulator {
    pub fn new(rule: &Rule) -> Self {
        Self {
            rule: *rule,
            number_of_players: 0,
            seat_order: 0,
            current_game_phase: GamePhase::WaitForPlayerSeat,
            shoe: shoe::Shoe::new(rule.number_of_decks, rule.cut_card_proportion),
            dealer_up_card: Default::default(),
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
        if number_of_players == 0 && seat_order == 0 {
            return Ok(());
        }
        self.number_of_players = number_of_players;
        self.seat_order = seat_order;
        Ok(())
    }

    /// Can be called at PlaceBets phase.
    /// Place 0 bet to indicate not to place any bet this time.
    pub fn place_bets(&mut self, bet: u32) {
        self.current_hand.set_original_bet(bet);
    }
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
            payout_insurance: 3.0,
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

// // Bet 100
// fn play_a_round<T: Strategy>(rule: &Rule, strategy: &mut T, shoe: &mut Shoe) -> (i32, bool) {
//     let (my_first_card, _) = shoe.deal_card();
//     let (dealer_up_card, _) = shoe.deal_card();
//     let (my_second_card, _) = shoe.deal_card();
//     let mut counts = [0; 10];
//     for i in shoe.current_index..shoe.cards.len() {
//         counts[(shoe.cards[i] - 1) as usize] += 1;
//     }
//     let initial_shoe = CardCount::new(&counts);

//     let initial_situation = InitialSituation::new(
//         initial_shoe,
//         (my_first_card, my_second_card),
//         dealer_up_card,
//     );
//     strategy.init(rule, &initial_situation);
//     let (dealer_hole_card, _) = shoe.deal_card();

//     let mut current_hand = CardCount::new(&[0; 10]);
//     current_hand.add_card(my_first_card);
//     current_hand.add_card(my_second_card);

//     let dealer_natural_blackjack =
//         dealer_up_card + dealer_hole_card == 11 && (dealer_up_card == 1 || dealer_hole_card == 1);
//     let me_natural_blackjack = current_hand.get_sum() == 11 && current_hand.is_soft();

//     if dealer_natural_blackjack {
//         if me_natural_blackjack {
//             return (0, shoe.reached_cut_card());
//         }
//         return (-100, shoe.reached_cut_card());
//     }

//     let mut bet = 100;
//     let mut has_surrendered = false;
//     loop {
//         if current_hand.get_sum() > 21 {
//             break;
//         }
//         let my_decision = strategy.make_decision(&current_hand);
//         print!("{:#?} ", my_decision);
//         match my_decision {
//             Decision::Hit => {
//                 let (card, _) = shoe.deal_card();
//                 current_hand.add_card(card);
//             }
//             Decision::Stand => {
//                 break;
//             }
//             Decision::Double => {
//                 let (card, _) = shoe.deal_card();
//                 current_hand.add_card(card);
//                 bet *= 2;
//                 break;
//             }
//             Decision::Surrender => {
//                 has_surrendered = true;
//                 break;
//             }
//             _ => {
//                 panic!("wtf??")
//             }
//         }
//     }
//     println!();

//     let my_sum = {
//         if current_hand.is_soft() && current_hand.get_sum() + 10 <= 21 {
//             current_hand.get_sum() + 10
//         } else {
//             current_hand.get_sum()
//         }
//     };

//     let mut dealer_sum = dealer_up_card + dealer_hole_card;
//     let mut dealer_soft = dealer_up_card == 1 || dealer_hole_card == 1;
//     while !(dealer_sum >= 17 || dealer_soft && dealer_sum + 10 > 17 && dealer_sum + 10 <= 21) {
//         let (card, _) = shoe.deal_card();
//         dealer_soft = dealer_soft || card == 1;
//         dealer_sum += card;
//     }
//     if dealer_sum < 17 {
//         dealer_sum += 10;
//     }
//     let dealer_sum = dealer_sum as u16;

//     if has_surrendered {
//         bet = -bet / 2;
//     } else if my_sum > 21 {
//         bet = -bet;
//     } else if me_natural_blackjack {
//         bet += bet / 2;
//     } else if dealer_sum <= 21 {
//         if my_sum < dealer_sum {
//             bet = -bet;
//         } else if my_sum == dealer_sum {
//             bet = 0;
//         }
//     }

//     (bet, shoe.reached_cut_card())
// }

// fn get_typical_rule() -> Rule {
//     Rule {
//         number_of_decks: 8,
//         cut_card_proportion: 0.5,
//         split_all_limits: 1,
//         split_ace_limits: 1,
//         double_policy: crate::DoublePolicy::AnyTwo,
//         dealer_hit_on_soft17: true,
//         allow_das: true,
//         allow_late_surrender: true,
//         peek_policy: crate::PeekPolicy::UpAceOrTen,

//         payout_blackjack: 1.5,
//         payout_insurance: 0.0,
//     }
// }

// #[test]
// fn test_strategy_on_new_shoe() {
//     println!("Test begin!!!");
//     let firsts = vec![1, 5, 2];
//     let mut shoe = Shoe::new(8, 0.5, &firsts);
//     let rule = get_typical_rule();
//     let mut basic_strategy: BasicStrategy = Default::default();
//     let mut my_strategy: MyStrategy = MyStrategy {
//         rule: rule,
//         sol: SolutionForInitialSituation {
//             general_solution: StateArray::new(),
//             split_expectation: 0.0,
//         },
//     };

//     let mut acc_basic: i32 = 0;
//     let mut acc_my: i32 = 0;
//     let total_rounds = 1_000_000;

//     let mut duration_max: u128 = 0;
//     let mut duration_min: u128 = u128::MAX;
//     let mut duration_total: u128 = 0;
//     for round in 0..total_rounds {
//         shoe.reinit(firsts.len());
//         while shoe.cards[0] == 1 && shoe.cards[2] == 1 {
//             shoe.reinit(firsts.len());
//         }
//         // shoe.cards[0] = 1;
//         // shoe.cards[1] = 9;
//         // shoe.cards[2] = 2;
//         // shoe.cards[3] = 10;
//         // shoe.cards[4] = 8;
//         // shoe.cards[5] = 10;
//         // shoe.cards[6] = 7;
//         // shoe.cards[7] = 2;
//         // shoe.cards[8] = 9;
//         // shoe.cards[9] = 2;
//         // shoe.cards[10] = 10;
//         print!("Turn #{}: ", round);
//         for i in 0..20 {
//             print!("{} ", shoe.cards[i]);
//         }
//         println!();
//         let (profit_basic, _) = play_a_round(&rule, &mut basic_strategy, &mut shoe);
//         acc_basic += profit_basic;
//         shoe.retry();
//         let time_start = SystemTime::now();
//         let (profit_my, _) = play_a_round(&rule, &mut my_strategy, &mut shoe);
//         let duration = SystemTime::now()
//             .duration_since(time_start)
//             .unwrap()
//             .as_millis();
//         if duration_max < duration {
//             duration_max = duration;
//         }
//         if duration_min > duration {
//             duration_min = duration;
//         }
//         duration_total += duration;
//         acc_my += profit_my;
//         println!(
//             "Turn #{}: {:#?}, {:#?} this({:.2}s) max({:.2}s) avg({:.2}s) min({:.2}s)",
//             round,
//             acc_basic,
//             acc_my,
//             duration as f64 / 1000.0,
//             duration_max as f64 / 1000.0,
//             duration_total as f64 / (round + 1) as f64 / 1000.0,
//             duration_min as f64 / 1000.0,
//         );
//     }
//     println!();
//     println!("Acc: {}, {}", acc_basic, acc_my);
//     println!("Total rounds: {}", total_rounds);
// }
