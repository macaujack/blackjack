use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{AddAssign, Index, IndexMut, SubAssign};

const MOD: u128 = 3817949514078926267; // A prime number with 62 bits.
const BASE: u128 = 211;
const POW_BASE: [u128; 10] = get_powers_of_base();

const fn get_powers_of_base() -> [u128; 10] {
    let mut ret: [u128; 10] = [0; 10];
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
#[derive(Debug, Default, Clone)]
pub struct SingleStateArray<T: Default> {
    data: HashMap<u128, T>,
}

impl<T: Default> SingleStateArray<T> {
    pub fn new() -> SingleStateArray<T> {
        SingleStateArray {
            data: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: HashMap::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn contains_state(&self, index: &CardCount) -> bool {
        self.data.contains_key(&index.hash_value)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl<T: Default> Index<&CardCount> for SingleStateArray<T> {
    type Output = T;
    fn index(&self, index: &CardCount) -> &Self::Output {
        &self.data[&index.hash_value]
    }
}

impl<T: Default> IndexMut<&CardCount> for SingleStateArray<T> {
    fn index_mut(&mut self, index: &CardCount) -> &mut Self::Output {
        if !self.data.contains_key(&index.hash_value) {
            self.data.insert(index.hash_value, Default::default());
        }

        self.data.get_mut(&index.hash_value).unwrap()
    }
}

/// This struct provide a convenient way to use 2 CardCount structs as the index of the
/// array.
#[derive(Debug, Default, Clone)]
pub struct DoubleStateArray<T: Default> {
    data: HashMap<u128, T>,
}

impl<T: Default> DoubleStateArray<T> {
    pub fn new() -> DoubleStateArray<T> {
        DoubleStateArray {
            data: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: HashMap::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn contains_state(&self, index: DoubleCardCountIndex) -> bool {
        self.data.contains_key(&index.hash_value)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl<T: Default> Index<DoubleCardCountIndex> for DoubleStateArray<T> {
    type Output = T;
    fn index(&self, index: DoubleCardCountIndex) -> &Self::Output {
        &self.data[&index.hash_value]
    }
}

impl<T: Default> IndexMut<DoubleCardCountIndex> for DoubleStateArray<T> {
    fn index_mut(&mut self, index: DoubleCardCountIndex) -> &mut Self::Output {
        let hash = index.hash_value;
        if !self.data.contains_key(&hash) {
            self.data.insert(hash, Default::default());
        }

        self.data.get_mut(&hash).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct CardCount {
    counts: [u16; 10],
    hash_value: u128,
    sum: u16,
    total: u16,
}

/// This provides a container to store the numbers of each card value (from
/// 1 to 10 inclusive).
impl CardCount {
    pub fn new(counts: &[u16; 10]) -> CardCount {
        let mut card_count = CardCount {
            counts: *counts,
            hash_value: 0,
            sum: 0,
            total: 0,
        };

        card_count.propagate_counts();

        card_count
    }

    pub fn with_number_of_decks(number_of_decks: u8) -> CardCount {
        let mut counts = [(number_of_decks * 4) as u16; 10];
        counts[9] = (number_of_decks * 16) as u16;
        Self::new(&counts)
    }

    /// Add a card of given card value.
    ///
    /// Note that this method won't check if the card value is valid.
    pub fn add_card(&mut self, card_value: u8) {
        let index = (card_value - 1) as usize;
        self.counts[index] += 1;
        self.hash_value = (self.hash_value + POW_BASE[index]) % MOD;
        self.sum += card_value as u16;
        self.total += 1;
    }

    /// Remove a card of given card value.
    ///
    /// Note that this method won't check if the card value is valid. It also
    /// won't check if the number of the given card value is already 0.
    pub fn remove_card(&mut self, card_value: u8) {
        let index = (card_value - 1) as usize;
        self.counts[index] -= 1;
        self.hash_value = (self.hash_value + MOD - POW_BASE[index]) % MOD;
        self.sum -= card_value as u16;
        self.total -= 1;
    }

    /// Note that this method treats Ace as 1.
    pub fn get_sum(&self) -> u16 {
        self.sum
    }

    pub fn get_total(&self) -> u16 {
        self.total
    }

    pub fn is_soft(&self) -> bool {
        self.counts[0] > 0
    }

    pub fn bust(&self) -> bool {
        self.sum > 21
    }

    pub fn is_natural(&self) -> bool {
        self.total == 2 && self.counts[0] == 1 && self.counts[9] == 1
    }

    pub fn get_actual_sum(&self) -> u16 {
        if self.is_soft() && self.sum + 10 <= 21 {
            self.sum + 10
        } else {
            self.sum
        }
    }

    /// Used to only calculate the hash value after adding. The other fields are
    /// all in invalid states and cannot be used.
    pub fn fast_add_assign(&mut self, rhs: &Self) {
        self.hash_value = (self.hash_value + rhs.hash_value) % MOD;
    }

    /// Used to only calculate the hash value after substracting. The other fields are
    /// all in invalid states and cannot be used.
    pub fn fast_sub_assign(&mut self, rhs: &Self) {
        self.hash_value = (self.hash_value + MOD - rhs.hash_value) % MOD;
    }

    fn propagate_counts(&mut self) {
        self.hash_value = 0;
        self.sum = 0;
        self.total = 0;
        for i in 0..self.counts.len() {
            self.hash_value += (self.counts[i] as u128) * POW_BASE[i];
            self.sum += ((i + 1) as u16) * self.counts[i];
            self.total += self.counts[i] as u16;
        }
        self.hash_value %= MOD;
    }
}

impl SubAssign<&CardCount> for CardCount {
    fn sub_assign(&mut self, rhs: &CardCount) {
        for i in 0..self.counts.len() {
            self.counts[i] -= rhs.counts[i];
        }

        self.hash_value = (self.hash_value + MOD - rhs.hash_value) % MOD;
        self.total -= rhs.total;
        self.sum -= rhs.sum;
    }
}

impl AddAssign<&CardCount> for CardCount {
    fn add_assign(&mut self, rhs: &CardCount) {
        for i in 0..self.counts.len() {
            self.counts[i] += rhs.counts[i];
        }

        self.hash_value = (self.hash_value + rhs.hash_value) % MOD;
        self.total += rhs.total;
        self.sum += rhs.sum;
    }
}

impl Index<u8> for CardCount {
    type Output = u16;
    fn index(&self, index: u8) -> &Self::Output {
        &self.counts[(index - 1) as usize]
    }
}

impl Hash for CardCount {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u128(self.hash_value);
    }
}

impl PartialEq for CardCount {
    fn eq(&self, other: &Self) -> bool {
        return self.hash_value == other.hash_value;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandState {
    PlaceHolder = 0,
    Normal,
    Surrender,
    Double,
}

impl Default for HandState {
    fn default() -> Self {
        HandState::PlaceHolder
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DoubleCardCountIndex {
    hash_value: u128,
}

impl DoubleCardCountIndex {
    pub fn new(hand0: &CardCount, mul0: HandState, hand1: &CardCount) -> Self {
        Self {
            hash_value: hand0.hash_value | ((mul0 as u128) << 62) | (hand1.hash_value << 64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    fn generate_random_counts(number_of_decks: u8) -> [u16; 10] {
        let mut rng = rand::thread_rng();
        let mut counts: [u16; 10] = [0; 10];
        for i in 0..9 {
            counts[i] = rng.gen_range(0..=(number_of_decks as u16) * 4);
        }
        counts[9] = rng.gen_range(0..=(number_of_decks as u16) * 16);

        counts
    }

    fn horner_method(counts: &[u16; 10]) -> u128 {
        let mut ret: u128 = 0;
        for i in (0..10).rev() {
            ret = (ret * BASE + (counts[i] as u128)) % MOD;
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

            if card_count[card_value] < (number_of_decks as u16) * 4
                || card_value == 10 && card_count[card_value] < (number_of_decks as u16) * 16
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
            let raw_counts: [u16; 10] = raw_counts;

            let mut sa: SingleStateArray<i32> = SingleStateArray::new();
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

    #[test]
    fn test_card_count_add_sub_assign() {
        let mut cc1 = CardCount::new(&[1, 2, 3, 0, 0, 0, 0, 0, 5, 6]);
        let original_hash_value = cc1.hash_value;
        assert_eq!(cc1.get_total(), 17);
        assert_eq!(cc1.get_sum(), 119);
        let cc2 = CardCount::new(&[10, 10, 10, 0, 6, 0, 0, 0, 20, 20]);
        cc1 += &cc2;
        assert_eq!(cc1.hash_value, horner_method(&cc1.counts));
        assert_eq!(cc1.get_total(), 17 + cc2.get_total());
        assert_eq!(cc1.get_sum(), 119 + cc2.get_sum());
        cc1 -= &cc2;
        assert_eq!(cc1.get_total(), 17);
        assert_eq!(cc1.get_sum(), 119);
        assert_eq!(cc1.hash_value, original_hash_value);
    }
}
