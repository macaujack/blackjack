use crate::CardCount;

use super::Card;

/// Represents all hand cards. May contain more than 1 group of cards because of split.
#[derive(Debug)]
pub struct Hand {
    group_bet_pairs: Vec<GroupBetPair>,
}

impl Hand {
    pub fn new() -> Hand {
        let group_bet_pair = GroupBetPair {
            group: Group::new(),
            bet: 0,
            win_already_determined: false,
        };
        Hand {
            group_bet_pairs: vec![group_bet_pair],
        }
    }

    /// The given group receives a given card.
    pub fn receive_card(&mut self, group_index: usize, card: Card) {
        self.group_bet_pairs[group_index].group.receive_card(card);
    }

    /// Splits the given group.
    pub fn split_group(&mut self, group_index: usize) {
        let mut new_group = Group::new();
        let card = self.group_bet_pairs[group_index].group.remove_card();
        new_group.receive_card(card);
        self.group_bet_pairs.push(GroupBetPair {
            group: new_group,
            bet: self.group_bet_pairs[group_index].bet,
            win_already_determined: false,
        });
    }

    /// Doubles down the given group.
    pub fn double_down(&mut self, group_index: usize) {
        self.group_bet_pairs[group_index].bet *= 2;
    }

    pub fn get_number_of_groups(&self) -> usize {
        self.group_bet_pairs.len()
    }

    pub fn get_bet(&self, group_index: usize) -> u32 {
        self.group_bet_pairs[group_index].bet
    }

    pub fn set_original_bet(&mut self, bet: u32) {
        self.group_bet_pairs[0].bet = bet;
    }

    pub fn determine_winning(&mut self, group_index: usize, multiplier: f64) {
        self.group_bet_pairs[group_index].win_already_determined = true;
        let winning = (self.group_bet_pairs[group_index].bet as f64 * multiplier) as u32;
        self.group_bet_pairs[group_index].bet = winning;
    }

    pub fn is_winning_already_determined(&self, group_index: usize) -> bool {
        self.group_bet_pairs[group_index].win_already_determined
    }

    pub fn get_cards(&self, group_index: usize) -> &Vec<Card> {
        &self.group_bet_pairs[group_index].group.cards
    }

    pub fn get_card_counts(&self, group_index: usize) -> &CardCount {
        &self.group_bet_pairs[group_index].group.card_count
    }

    /// Clears all the cards in all groups. Remove all the extra groups (i.e., groups
    /// that come from split), leaving only 1 original group, and it is empty.
    pub fn clear(&mut self) {
        while self.group_bet_pairs.len() > 1 {
            self.group_bet_pairs.pop();
        }
        self.group_bet_pairs[0].group.clear();
        self.group_bet_pairs[0].bet = 0;
        self.group_bet_pairs[0].win_already_determined = false;
    }
}

#[derive(Debug)]
struct Group {
    cards: Vec<Card>,
    card_count: CardCount,
}

impl Group {
    fn new() -> Self {
        Self {
            cards: Vec::with_capacity(3),
            card_count: CardCount::with_number_of_decks(0),
        }
    }

    fn receive_card(&mut self, card: Card) {
        self.cards.push(card);
        self.card_count.add_card(card.blackjack_value());
    }

    fn remove_card(&mut self) -> Card {
        let card = self.cards.pop().unwrap();
        self.card_count.remove_card(card.blackjack_value());
        card
    }

    fn clear(&mut self) {
        self.cards.clear();
        self.card_count = CardCount::with_number_of_decks(0);
    }
}

#[derive(Debug)]
struct GroupBetPair {
    group: Group,
    bet: u32,
    /// Indicate whether the winning money of this group has already been determined. This happens
    /// when you bust, surrender or reach Charlie number.
    win_already_determined: bool,
}

#[cfg(test)]
mod tests {
    use crate::simulation::Suit;

    use super::*;

    #[test]
    fn should_split_successfully() {
        let mut hand = Hand::new();
        hand.receive_card(
            0,
            Card {
                face_value: 8,
                suit: Suit::Diamond,
            },
        );
        hand.receive_card(
            0,
            Card {
                face_value: 8,
                suit: Suit::Club,
            },
        );
        hand.split_group(0);
        assert_eq!(hand.get_number_of_groups(), 2);
        assert_eq!(
            hand.group_bet_pairs[0].group.cards[0],
            Card {
                face_value: 8,
                suit: Suit::Diamond
            }
        );
        assert_eq!(
            hand.group_bet_pairs[1].group.cards[0],
            Card {
                face_value: 8,
                suit: Suit::Club,
            }
        );
    }
}
