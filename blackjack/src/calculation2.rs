mod dealer_play;
mod util;

use crate::Decision;

#[derive(Debug, Clone)]
pub struct ExpectationAll {
    pub hit: f64,
    pub stand: f64,
    pub double: f64,
    pub surrender: f64,
    pub split: f64,

    pub insurance: f64,

    pub summary: f64,
}

impl Default for ExpectationAll {
    fn default() -> Self {
        Self {
            hit: -f64::INFINITY,
            stand: -f64::INFINITY,
            double: -f64::INFINITY,
            surrender: -f64::INFINITY,
            split: -f64::INFINITY,

            insurance: -f64::INFINITY,

            summary: -f64::INFINITY,
        }
    }
}

impl ExpectationAll {
    pub fn get_max_expectation(&self) -> (f64, Decision) {
        let (mut mx_ex, mut decision) = (self.hit, Decision::Hit);
        if mx_ex < self.stand {
            (mx_ex, decision) = (self.stand, Decision::Stand);
        }
        if mx_ex < self.double {
            (mx_ex, decision) = (self.double, Decision::Double);
        }
        if mx_ex < self.surrender {
            (mx_ex, decision) = (self.surrender, Decision::Surrender);
        }
        if mx_ex < self.split {
            (mx_ex, decision) = (self.split, Decision::Split);
        }

        (mx_ex, decision)
    }
}
