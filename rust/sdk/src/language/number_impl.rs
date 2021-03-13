use super::Number;
use Number::*;

impl Number {
    fn as_f64(&self) -> f64 {
        match self {
            Decimal(d) => *d,
            Natural(n) => *n as f64,
        }
    }

    pub fn add(&self, other: &Number) -> Number {
        match (self, other) {
            (Natural(l), Natural(r)) => Natural(l.saturating_add(*r)),
            (l, r) => Decimal(l.as_f64() + r.as_f64()),
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Natural(l), Natural(r)) => l.partial_cmp(r),
            (l, r) => l.as_f64().partial_cmp(&r.as_f64()),
        }
    }
}
