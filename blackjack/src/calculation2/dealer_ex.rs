use crate::{CardCount, PeekPolicy, Rule, SingleStateArray};

#[derive(Debug, Clone, Default)]
pub struct DealerHandValueProbability {
    // 0 for Bust.
    // [1, 5] for [17, 21].
    // Probability of natural Blackjack = 1.0 - probabilies_prefix_sum[5].
    probabilities_prefix_sum: [f64; 6],
}

impl DealerHandValueProbability {
    pub fn p_worse_than_player(&self, player_actual_sum: u16) -> f64 {
        let x = player_actual_sum as usize;
        match x {
            0..=17 => self.probabilities_prefix_sum[0],
            18..=21 => self.probabilities_prefix_sum[x - 17],
            _ => panic!("Impossible to reach"),
        }
    }

    pub fn p_better_than_player(&self, player_actual_sum: u16) -> f64 {
        let x = player_actual_sum as usize;
        match x {
            0..=16 => 1.0 - self.probabilities_prefix_sum[0],
            17..=21 => 1.0 - self.probabilities_prefix_sum[x - 16],
            _ => panic!("Impossible to reach"),
        }
    }

    fn end_with_bust(&mut self) {
        for p in self.probabilities_prefix_sum.iter_mut() {
            *p = 1.0;
        }
    }

    fn end_with_normal(&mut self, dealer_actual_sum: u16) {
        for i in (dealer_actual_sum - 16) as usize..self.probabilities_prefix_sum.len() {
            self.probabilities_prefix_sum[i] = 1.0;
        }
    }

    fn end_with_natural(&mut self) {}

    fn add_assign_with_p(&mut self, rhs: &Self, p: f64) {
        for i in 0..self.probabilities_prefix_sum.len() {
            self.probabilities_prefix_sum[i] += rhs.probabilities_prefix_sum[i] * p;
        }
    }
}

#[derive(Clone)]
struct MutPointer {
    ptr: *mut SingleStateArray<DealerHandValueProbability>,
}

impl MutPointer {
    fn new(ptr: *mut SingleStateArray<DealerHandValueProbability>) -> Self {
        Self { ptr }
    }
}

impl Default for MutPointer {
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }
}

type MemoizationFunctionType =
    fn(&Rule, &CardCount, &mut CardCount, &mut SingleStateArray<DealerHandValueProbability>);

pub struct DealerPlay<'a> {
    rule: &'a Rule,
    number_of_threads: usize,
    memoization_functions: [MemoizationFunctionType; 10],

    dealer_odds: SingleStateArray<MutPointer>,
    dealer_odds_memory_pool: Vec<SingleStateArray<DealerHandValueProbability>>,
    dealer_odds_memory_pool_index: usize,
    dealer_odd_already_calculated: SingleStateArray<()>,

    dealer_hands_aux: Vec<CardCount>,
    dealer_hands_with_up_cards: Vec<CardCount>,
}

impl<'a> DealerPlay<'a> {
    const MEMORY_POOL_SIZE: usize = 10000;

    pub fn new(rule: &'a Rule, number_of_threads: usize) -> Self {
        let memoization_function_for_ace = match rule.peek_policy {
            PeekPolicy::UpAce | PeekPolicy::UpAceOrTen => Self::memoization_dealer_gets_cards::<10>,
            _ => Self::memoization_dealer_gets_cards::<0>,
        };
        let memoization_function_for_ten = match rule.peek_policy {
            PeekPolicy::UpAceOrTen => Self::memoization_dealer_gets_cards::<1>,
            _ => Self::memoization_dealer_gets_cards::<0>,
        };

        let mut dealer_hands_with_up_cards = Vec::with_capacity(10);
        for dealer_up_card in 1..=10 {
            let mut dealer_hand = CardCount::with_number_of_decks(0);
            dealer_hand.add_card(dealer_up_card);
            dealer_hands_with_up_cards.push(dealer_hand);
        }

        Self {
            rule,
            number_of_threads,
            memoization_functions: [
                memoization_function_for_ace,
                Self::memoization_dealer_gets_cards::<0>,
                Self::memoization_dealer_gets_cards::<0>,
                Self::memoization_dealer_gets_cards::<0>,
                Self::memoization_dealer_gets_cards::<0>,
                Self::memoization_dealer_gets_cards::<0>,
                Self::memoization_dealer_gets_cards::<0>,
                Self::memoization_dealer_gets_cards::<0>,
                Self::memoization_dealer_gets_cards::<0>,
                memoization_function_for_ten,
            ],

            dealer_odds: Default::default(),
            dealer_odds_memory_pool: Vec::with_capacity(10000),
            dealer_odds_memory_pool_index: 0,
            dealer_odd_already_calculated: Default::default(),

            dealer_hands_aux: vec![CardCount::with_number_of_decks(0); 10],
            dealer_hands_with_up_cards,
        }
    }

    pub fn get_dealer_hand_value_probability(
        &self,
        dealer_plus_shoe: &CardCount,
        dealer_up_card: u8,
    ) -> &DealerHandValueProbability {
        let odd = self.dealer_odds[dealer_plus_shoe].ptr;
        let odd = unsafe { &*odd };
        &odd[&self.dealer_hands_with_up_cards[(dealer_up_card - 1) as usize]]
    }

    pub fn clear_dealer_odds(&mut self) {
        self.dealer_odds.clear();
        self.dealer_odd_already_calculated.clear();
    }

    pub fn update_dealer_odds(&mut self, dealer_plus_shoes: &[CardCount]) {
        for dealer_plus_shoe in dealer_plus_shoes {
            if self
                .dealer_odd_already_calculated
                .contains_state(dealer_plus_shoe)
            {
                continue;
            }
            self.dealer_odd_already_calculated[dealer_plus_shoe] = ();

            // Allocate a new odd from memory pool.
            let odd = &mut self.dealer_odds_memory_pool[self.dealer_odds_memory_pool_index];
            odd.clear();
            self.dealer_odds_memory_pool_index =
                (self.dealer_odds_memory_pool_index + 1) % Self::MEMORY_POOL_SIZE;

            Self::update_dealer_odd(
                self.rule,
                dealer_plus_shoe,
                &self.memoization_functions,
                &mut self.dealer_hands_aux[0],
                odd,
            );

            self.dealer_odds[dealer_plus_shoe] =
                MutPointer::new(odd as *mut SingleStateArray<DealerHandValueProbability>);
        }
        // TODO: Use multithreads
    }

    fn update_dealer_odd(
        // Input parameters
        rule: &Rule,
        dealer_plus_shoe: &CardCount,
        memoization_functions: &[MemoizationFunctionType; 10],

        // Parameters to maintain current state
        dealer_hand: &mut CardCount,

        // Output parameters
        odd: &mut SingleStateArray<DealerHandValueProbability>,
    ) {
        for dealer_up_card in 1..=10 {
            if dealer_plus_shoe[dealer_up_card] == 0 {
                continue;
            }
            dealer_hand.add_card(dealer_up_card);

            let memoization_function = memoization_functions[(dealer_up_card - 1) as usize];
            memoization_function(rule, dealer_plus_shoe, dealer_hand, odd);

            dealer_hand.remove_card(dealer_up_card);
        }
    }

    fn memoization_dealer_gets_cards<const IMPOSSIBLE_DEALER_HOLE_CARD: u8>(
        // Input parameters
        rule: &Rule,
        dealer_plus_shoe: &CardCount,

        // Parameters to maintain current state
        dealer_hand: &mut CardCount,

        // Output parameters
        odd: &mut SingleStateArray<DealerHandValueProbability>,
    ) {
        if odd.contains_state(dealer_hand) {
            return;
        }
        odd[dealer_hand] = Default::default();

        // Case 1: Dealer must stand.
        if dealer_hand.bust() {
            odd[dealer_hand].end_with_bust();
            return;
        }
        let actual_sum = dealer_hand.get_actual_sum();
        if actual_sum > 17 {
            if dealer_hand.is_natural() {
                odd[dealer_hand].end_with_bust();
            } else {
                odd[dealer_hand].end_with_normal(actual_sum);
            }
            return;
        }
        if actual_sum == 17 {
            if !dealer_hand.is_soft() || !rule.dealer_hit_on_soft17 {
                odd[dealer_hand].end_with_normal(17);
                return;
            }
        }

        // Case 2: Dealer must hit.
        let impossible_card_number = {
            if IMPOSSIBLE_DEALER_HOLE_CARD == 0 {
                0
            } else {
                // This is impossible to be 0, because this means that all the cards in the shoe has been
                // dealt, which is impossible to happen.
                dealer_plus_shoe[IMPOSSIBLE_DEALER_HOLE_CARD]
                    - dealer_hand[IMPOSSIBLE_DEALER_HOLE_CARD]
            }
        };
        let current_valid_shoe_total =
            dealer_plus_shoe.get_total() - dealer_hand.get_total() - impossible_card_number;
        let current_valid_shoe_total = current_valid_shoe_total as f64;

        let (next_card_min, next_card_max) = match IMPOSSIBLE_DEALER_HOLE_CARD {
            0 => (1, 10),
            1 => (2, 10),
            10 => (1, 9),
            _ => panic!("Impossible to reach"),
        };

        for card_value in next_card_min..=next_card_max {
            if dealer_hand[card_value] == dealer_plus_shoe[card_value] {
                continue;
            }

            dealer_hand.add_card(card_value);

            Self::memoization_dealer_gets_cards::<0>(rule, dealer_plus_shoe, dealer_hand, odd);
            let next_state_odds = &odd[dealer_hand] as *const DealerHandValueProbability;

            dealer_hand.remove_card(card_value);

            let target_cards_in_shoe = dealer_plus_shoe[card_value] - dealer_hand[card_value];
            let p = target_cards_in_shoe as f64 / current_valid_shoe_total;
            unsafe {
                // Here, we know that we are referencing 2 different pieces of memory, but
                // compilier doesn't know. This is absolutely safe.
                odd[dealer_hand].add_assign_with_p(&*next_state_odds, p);
            }
        }
    }
}
