mod statearray;

pub use statearray::CardCount;
pub use statearray::StateArray;

pub struct Rule {
    pub number_of_decks: u8,
    pub double_policy: DoublePolicy,
    pub dealer_hit_on_soft17: bool,
    pub allow_das: bool,
    pub allow_late_surrender: bool,
    pub dealer_peek_hole_card: bool,
}

pub enum DoublePolicy {
    AnyTwo,
    NineTenElevenOnly,
    TenElevenOnly,
}

pub struct SingleHandState {}
