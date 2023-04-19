use crate::CardCount;

use super::{Card, Suit};

use strum::IntoEnumIterator;

use rand::seq::SliceRandom;
use rand::thread_rng;

/// Represents a shoe in the real world.
#[derive(Debug, Clone)]
pub struct Shoe {
    number_of_decks: u8,
    cut_card_index: usize,
    cards: Vec<Card>,
    card_count: CardCount,
    current_index: usize,
}

impl Shoe {
    /// Creates a new shoe with ordered cards.
    pub fn new(number_of_decks: u8, cut_card_proportion: f64) -> Shoe {
        let mut cards = Vec::with_capacity(number_of_decks as usize * 52);
        for _ in 0..number_of_decks {
            for suit in Suit::iter() {
                for face_value in 1..=13 {
                    cards.push(Card { face_value, suit });
                }
            }
        }
        Shoe {
            number_of_decks,
            cut_card_index: (cut_card_proportion * (number_of_decks as u16 * 52) as f64) as usize,
            cards,
            card_count: CardCount::with_number_of_decks(number_of_decks),
            current_index: 0,
        }
    }

    /// Returns the dealt cards back into the shoe, and shuffles. This method makes sure the given first few cards
    /// will be at the frontmost positions of the shoe. Panics if requirement cannot be met.
    /// Note that the cards are given in blackjack values (i.e., 1 stands for A. 10 stands for 10 and J, Q, K).
    pub fn shuffle_with_firsts(&mut self, firsts: &Vec<u8>) {
        let mut counts = [self.number_of_decks; 52];
        self.current_index = 0;
        self.card_count = CardCount::with_number_of_decks(self.number_of_decks);

        let mut idx = 0;
        for blackjack_value in firsts {
            let card_integer = find_suitable_card(&counts, *blackjack_value)
                .expect("The given first cards are invalid");
            counts[card_integer as usize] -= 1;
            self.cards[idx] = Card::try_from(card_integer).unwrap();
            idx += 1;
        }

        for suit in Suit::iter() {
            for face_value in 1..=13 {
                let card = Card { face_value, suit };
                let card_integer: u8 = card.into();
                for _ in 0..counts[card_integer as usize] {
                    self.cards[idx] = card;
                    idx += 1;
                }
            }
        }

        self.cards[firsts.len()..].shuffle(&mut thread_rng());
    }

    /// Returns the dealt cards back into the shoe and shuffles. Panics if start_index out of bound.
    pub fn shuffle(&mut self, start_index: usize) {
        self.cards[start_index..].shuffle(&mut thread_rng());
        self.current_index = 0;
        self.card_count = CardCount::with_number_of_decks(self.number_of_decks);
    }

    /// Returns the dealt cards back into the shoe in the original order.
    pub fn retry(&mut self) {
        self.current_index = 0;
        self.card_count = CardCount::with_number_of_decks(self.number_of_decks);
    }

    /// Deals a card if the shoe is not empty. Returns None if empty.
    pub fn deal_card(&mut self) -> Option<Card> {
        self.current_index += 1;
        if self.current_index > self.cards.len() {
            None
        } else {
            let card = self.cards[self.current_index - 1];
            self.card_count.remove_card(card.blackjack_value());
            Some(card)
        }
    }

    /// Checks if the cut card has been reached.
    pub fn reached_cut_card(&self) -> bool {
        self.current_index >= self.cut_card_index
    }

    pub fn get_card_count(&self) -> CardCount {
        self.card_count
    }
}

fn find_suitable_card(counts: &[u8; 52], blackjack_value: u8) -> Result<u8, ()> {
    let mut card: Card = Default::default();
    let (lo, hi) = {
        if blackjack_value == 10 {
            (10, 13)
        } else {
            (blackjack_value, blackjack_value)
        }
    };

    for face_value in lo..=hi {
        card.face_value = face_value;
        for suit in Suit::iter() {
            card.suit = suit;
            let card: u8 = card.into();
            if counts[card as usize] > 0 {
                return Ok(card);
            }
        }
    }

    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number_of_cards_is_correct(shoe: &Shoe) -> bool {
        let mut counts = [0 as u8; 52];
        for card in &shoe.cards {
            let card_integer: u8 = (*card).into();
            counts[card_integer as usize] += 1;
        }

        for i in 0..52 {
            if counts[i] != shoe.number_of_decks as u8 {
                return false;
            }
        }
        true
    }

    #[test]
    fn new_shoe_is_ordered() {
        let number_of_decks = 3;
        let shoe = Shoe::new(number_of_decks, 0.3333333);
        assert!(number_of_cards_is_correct(&shoe));
        assert_eq!(shoe.cards.len(), number_of_decks as usize * 52);
        let mut card: Card = Default::default();
        for suit in Suit::iter() {
            card.suit = suit;
            for face_value in 1..=13 {
                card.face_value = face_value;
                let card_integer: u8 = card.into();
                for i in 0..number_of_decks {
                    assert_eq!(card, shoe.cards[card_integer as usize + 52 * i as usize]);
                }
            }
        }
    }

    #[test]
    fn test_shuffle_with_firsts() {
        let number_of_decks = 1;
        let mut shoe = Shoe::new(number_of_decks, 0.3333333);
        let mut firsts = vec![1, 2, 6, 6, 9];
        shoe.shuffle_with_firsts(&firsts);
        assert!(number_of_cards_is_correct(&shoe));
        for (i, blackjack_value) in firsts.iter().enumerate() {
            assert_eq!(shoe.cards[i].blackjack_value(), *blackjack_value);
        }

        firsts = vec![9, 10, 10, 10, 10, 10];
        shoe.shuffle_with_firsts(&firsts);
        assert!(number_of_cards_is_correct(&shoe));
        for (i, blackjack_value) in firsts.iter().enumerate() {
            assert_eq!(shoe.cards[i].blackjack_value(), *blackjack_value);
        }
    }

    #[test]
    #[should_panic]
    fn invalid_firsts_should_panic() {
        let number_of_decks = 1;
        let mut shoe = Shoe::new(number_of_decks, 0.3333333);
        let firsts = vec![1, 2, 6, 6, 9, 6, 6, 6];
        shoe.shuffle_with_firsts(&firsts);
    }

    #[test]
    #[should_panic]
    fn invalid_firsts_with_lots_of_ten_should_panic() {
        let number_of_decks = 2;
        let mut shoe = Shoe::new(number_of_decks, 0.3333333);
        let firsts = [10; 33].to_vec();
        shoe.shuffle_with_firsts(&firsts);
    }

    #[test]
    #[ignore]
    fn examine_shuffle_results() {
        let number_of_decks = 2;
        let mut shoe = Shoe::new(number_of_decks, 0.3333333);
        loop {
            shoe.shuffle(3);
            assert!(number_of_cards_is_correct(&shoe));
        }
    }

    #[test]
    fn card_count_is_correctly_synced() {
        let number_of_decks = 2;
        let mut shoe = Shoe::new(number_of_decks, 0.3333333);
        shoe.shuffle_with_firsts(&vec![1, 4, 4, 10]);
        _ = shoe.deal_card();
        assert_eq!(shoe.card_count[1], 7);
        _ = shoe.deal_card();
        assert_eq!(shoe.card_count[4], 7);
        _ = shoe.deal_card();
        assert_eq!(shoe.card_count[4], 6);
        _ = shoe.deal_card();
        assert_eq!(shoe.card_count[10], 31);
    }
}
