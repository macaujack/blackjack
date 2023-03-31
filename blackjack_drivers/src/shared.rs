use blackjack;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rule: ConfigRule,
    pub blackjack_simulator: ConfigBlackjackSimulator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRule {
    pub number_of_decks: u8,
    pub cut_card_proportion: f64,
    pub split_all_limits: u8,
    pub split_ace_limits: u8,
    pub double_policy: String,
    pub dealer_hit_on_soft17: bool,
    pub allow_das: bool,
    pub allow_late_surrender: bool,
    pub peek_policy: String,
    pub charlie_number: u8,

    pub payout_blackjack: f64,
    pub payout_insurance: f64,
}

impl TryInto<blackjack::Rule> for ConfigRule {
    type Error = serde::de::value::Error;

    fn try_into(self) -> Result<blackjack::Rule, Self::Error> {
        let blackjack_rule = blackjack::Rule {
            number_of_decks: self.number_of_decks,
            cut_card_proportion: self.cut_card_proportion,
            split_all_limits: self.split_all_limits,
            split_ace_limits: self.split_ace_limits,
            double_policy: self.double_policy.parse()?,
            dealer_hit_on_soft17: self.dealer_hit_on_soft17,
            allow_das: self.allow_das,
            allow_late_surrender: self.allow_late_surrender,
            peek_policy: self.peek_policy.parse()?,
            charlie_number: self.charlie_number,
            payout_blackjack: self.payout_blackjack,
            payout_insurance: self.payout_insurance,
        };

        Ok(blackjack_rule)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigBlackjackSimulator {
    pub number_of_threads: usize,
    pub games_in_period: u64,
}

/// Reads the content of a given config file and parses it to a Config.
///
/// Panics if any error occurs.
pub fn parse_config_from_file(filename: &str) -> Config {
    let file_content = fs::read_to_string(filename).unwrap();
    serde_yaml::from_str(&file_content).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_typical_config_rule() -> ConfigRule {
        ConfigRule {
            number_of_decks: 8,
            cut_card_proportion: 0.5,
            split_all_limits: 1,
            split_ace_limits: 1,
            double_policy: String::from("AnyTwo"),
            dealer_hit_on_soft17: false,
            allow_das: false,
            allow_late_surrender: false,
            peek_policy: String::from("UpAce"),
            charlie_number: 6,
            payout_blackjack: 1.5,
            payout_insurance: 2.0,
        }
    }

    #[test]
    fn can_convert_rule() {
        let config_rule = get_typical_config_rule();
        let converted_rule: blackjack::Rule = config_rule.try_into().unwrap();
        assert_eq!(converted_rule.number_of_decks, 8);
        assert_eq!(converted_rule.cut_card_proportion, 0.5);
        assert_eq!(
            converted_rule.double_policy,
            blackjack::DoublePolicy::AnyTwo
        );
        assert_eq!(converted_rule.peek_policy, blackjack::PeekPolicy::UpAce);
    }

    #[test]
    fn should_return_error_when_converting_rule() {
        let mut config_rule = get_typical_config_rule();
        config_rule.double_policy = String::from("Not a policy");
        let convert_result: Result<blackjack::Rule, serde::de::value::Error> =
            config_rule.try_into();
        assert!(convert_result.is_err());
    }
}
