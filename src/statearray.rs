use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Index, IndexMut};

const MOD: u64 = 1000000007;
const BASE: u64 = 211;
const POW_BASE: [u64; 10] = get_powers_of_base();

const fn get_powers_of_base() -> [u64; 10] {
    let mut ret: [u64; 10] = [0; 10];
    ret[0] = 1;

    let mut i = 1;
    while i < ret.len() {
        ret[i] = ret[i - 1] * BASE % MOD;
        i += 1;
    }

    ret
}

/// This struct provide a convenient way to use CardCount as the index of the
/// array.
#[derive(Debug)]
pub struct StateArray<T: Copy + Default> {
    data: HashMap<u64, T>,
}

impl<T: Copy + Default> StateArray<T> {
    pub fn new() -> StateArray<T> {
        StateArray {
            data: HashMap::new(),
        }
    }
}

impl<T: Copy + Default> Index<&CardCount> for StateArray<T> {
    type Output = T;
    fn index(&self, index: &CardCount) -> &Self::Output {
        &self.data[&index.hash_value]
    }
}

impl<T: Copy + Default> IndexMut<&CardCount> for StateArray<T> {
    fn index_mut(&mut self, index: &CardCount) -> &mut Self::Output {
        if !self.data.contains_key(&index.hash_value) {
            self.data.insert(index.hash_value, Default::default());
        }

        self.data.get_mut(&index.hash_value).unwrap()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CardCount {
    counts: [u8; 10],
    hash_value: u64,
}

/// This provides a container to store the numbers of each card value (from
/// 1 to 10 inclusive).
impl CardCount {
    pub fn new(counts: &[u8; 10]) -> CardCount {
        let mut card_count = CardCount {
            counts: *counts,
            hash_value: 0,
        };

        card_count.propagate_hash();

        card_count
    }

    /// Add a card of given card value.
    ///
    /// Note that this method won't check if the card value is valid.
    pub fn add_card(&mut self, card_value: u8) {
        let index = (card_value - 1) as usize;
        self.counts[index] += 1;
        self.hash_value = (self.hash_value + POW_BASE[index]) % MOD;
    }

    /// Remove a card of given card value.
    ///
    /// Note that this method won't check if the card value is valid. It also
    /// won't check if the number of the given card value is already 0.
    pub fn remove_card(&mut self, card_value: u8) {
        let index = (card_value - 1) as usize;
        self.counts[index] -= 1;
        self.hash_value = (self.hash_value + MOD - POW_BASE[index]) % MOD;
    }

    fn propagate_hash(&mut self) {
        self.hash_value = 0;
        for i in 0..self.counts.len() {
            self.hash_value += (self.counts[i] as u64) * POW_BASE[i];
        }
        self.hash_value %= MOD;
    }
}

impl Index<u8> for CardCount {
    type Output = u8;
    fn index(&self, index: u8) -> &Self::Output {
        &self.counts[(index - 1) as usize]
    }
}

impl Hash for CardCount {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash_value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    fn generate_random_counts(number_of_decks: u8) -> [u8; 10] {
        let mut rng = rand::thread_rng();
        let mut counts: [u8; 10] = [0; 10];
        for i in 0..9 {
            counts[i] = rng.gen_range(0..=number_of_decks * 4);
        }
        counts[9] = rng.gen_range(0..=number_of_decks * 16);

        counts
    }

    fn horner_method(counts: &[u8; 10]) -> u64 {
        let mut ret: u64 = 0;
        for i in (0..10).rev() {
            ret = (ret * BASE + (counts[i] as u64)) % MOD;
        }

        ret
    }

    #[test]
    fn hash_of_card_count() {
        for _turn in 0..10 {
            let counts = generate_random_counts(8);
            let gt_hash = horner_method(&counts);

            let card_count = CardCount::new(&counts);
            assert_eq!(card_count.hash_value, gt_hash);
        }
    }

    #[test]
    fn add_and_remove() {
        for _turn in 0..10 {
            let number_of_decks = 8;
            let mut counts = generate_random_counts(number_of_decks);
            let mut card_count = CardCount::new(&counts);
            let card_value: u8 = rand::thread_rng().gen_range(1..=10);

            if card_count[card_value] < number_of_decks * 4
                || card_value == 10 && card_count[card_value] < number_of_decks * 16
            {
                counts[(card_value - 1) as usize] += 1;
                card_count.add_card(card_value);

                assert_eq!(card_count.hash_value, horner_method(&counts));
            }
        }
    }

    #[test]
    fn test_state_array() {
        for _turn in 0..10 {
            let mut raw_counts = generate_random_counts(8);
            raw_counts[3] = 2;
            let raw_counts: [u8; 10] = raw_counts;

            let mut sa: StateArray<i32> = StateArray::new();
            let mut cc1 = CardCount::new(&raw_counts);
            sa[&cc1] = 666;
            cc1.add_card(3);
            sa[&cc1] = 111;
            let mut cc2 = CardCount::new(&raw_counts);
            assert_eq!(sa[&cc2], 666);
            cc2.add_card(3);
            assert_eq!(sa[&cc2], 111);
            cc2.remove_card(3);
            assert_eq!(sa[&cc2], 666);
        }
    }
}
