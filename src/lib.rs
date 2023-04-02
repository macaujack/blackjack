pub mod calculation;
pub mod simulation;
mod statearray;

pub use statearray::CardCount;
pub use statearray::StateArray;

pub struct Rule {
    pub number_of_decks: u8,
    pub cut_card_proportion: f64, // The proportion of cards before the cut card.
    pub split_all_limits: u8,     // Only supports 0 or 1 now.
    pub split_ace_limits: u8,     // Only supports 0 or 1 now.
    pub double_policy: DoublePolicy,
    pub dealer_hit_on_soft17: bool,
    pub allow_das: bool,
    pub allow_late_surrender: bool,
    pub dealer_peek_hole_card: bool,

    pub payout_blackjack: f64,
    pub payout_insurance: f64,
}

pub enum DoublePolicy {
    AnyTwo,
    NineTenElevenOnly,
    TenElevenOnly,
}

#[derive(Clone, Copy, Debug)]
pub struct InitialSituation {
    shoe: CardCount,
    hand_cards: (u8, u8),
    dealer_up_card: u8,
}

impl InitialSituation {
    pub fn new(shoe: CardCount, hand: (u8, u8), dealer_up_card: u8) -> Self {
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
