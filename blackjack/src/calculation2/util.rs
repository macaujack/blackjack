use crate::{CardCount, PeekPolicy, Rule};

pub fn get_card_probability(
    shoe: &CardCount,
    impossible_dealer_hole_card: u8,
    target_card: u8,
) -> f64 {
    let total = shoe.get_total() as f64;
    if shoe[target_card] == 0 {
        return 0.0;
    }
    let target_number = shoe[target_card] as f64;
    if impossible_dealer_hole_card == 0 {
        return target_number / total;
    }

    let p_hole_card_is_target_card = {
        if impossible_dealer_hole_card == target_card {
            0.0
        } else {
            target_number / (shoe.get_total() - shoe[impossible_dealer_hole_card]) as f64
        }
    };
    let shoe_total_minus_one = (shoe.get_total() - 1) as f64;
    let p1 = p_hole_card_is_target_card * (shoe[target_card].wrapping_sub(1)) as f64
        / shoe_total_minus_one;
    let p2 = (1.0 - p_hole_card_is_target_card) * target_number / shoe_total_minus_one;
    p1 + p2
}

pub fn get_impossible_dealer_hole_card(rule: &Rule, dealer_up_card: u8) -> u8 {
    match rule.peek_policy {
        PeekPolicy::UpAceOrTen => match dealer_up_card {
            1 => 10,
            10 => 1,
            _ => 0,
        },
        PeekPolicy::UpAce => match dealer_up_card {
            1 => 10,
            _ => 0,
        },
        PeekPolicy::NoPeek => 0,
    }
}
