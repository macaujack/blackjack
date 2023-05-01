pub mod calculation;
pub mod simulation;
mod statearray;

use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
pub use statearray::CardCount;
pub use statearray::StateArray;

#[derive(Clone, Copy)]
pub struct Rule {
    pub number_of_decks: u8,
    pub cut_card_proportion: f64, // The proportion of cards before the cut card. // TODO: Use this.
    pub split_all_limits: u8,     // Only supports 0 or 1 now. // TODO: Use this.
    pub split_ace_limits: u8,     // Only supports 0 or 1 now. // TODO: Use this.
    pub double_policy: DoublePolicy,
    pub dealer_hit_on_soft17: bool,
    pub allow_das: bool, // TODO: Use this.
    pub allow_late_surrender: bool,
    pub peek_policy: PeekPolicy,
    pub charlie_number: u8, // TODO: Use this.

    pub payout_blackjack: f64,
    pub payout_insurance: f64, // TODO: Use this.
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize_enum_str, Deserialize_enum_str)]
pub enum DoublePolicy {
    AnyTwo,
    NineTenElevenOnly,
    TenElevenOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize_enum_str, Deserialize_enum_str)]
pub enum PeekPolicy {
    UpAceOrTen,
    UpAce,
    NoPeek,
}

/// Stands for the initial situation. Note that in our model, the dealer deals herself the upcard
/// first, and then she deals 2 cards to player. This won't influence anything. And for this
/// reason, the player's two hand cards can be 0, indicating they are unknown yet.
#[derive(Clone, Copy, Debug)]
pub struct InitialSituation {
    shoe: CardCount,
    hand_cards: (u8, u8),
    dealer_up_card: u8,
}

impl InitialSituation {
    pub fn new(shoe: CardCount, hand: (u8, u8), dealer_up_card: u8) -> Self {
        if dealer_up_card == 0 || dealer_up_card > 10 {
            panic!("Invalid dealer up card! It must be in [1, 10]")
        }
        if hand.0 > 10 || hand.1 > 10 {
            panic!("Invalid hand card! It must be in [0, 10]")
        }
        InitialSituation {
            shoe,
            hand_cards: hand,
            dealer_up_card,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Decision {
    PlaceHolder,
    Hit,
    Stand,
    Double,
    Surrender,
    Split,
    Insurance,
}

impl Default for Decision {
    fn default() -> Self {
        Decision::PlaceHolder
    }
}
