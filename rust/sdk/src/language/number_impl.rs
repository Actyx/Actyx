use super::Number;
use num_traits::Pow;
use std::{cmp::Ordering, convert::TryInto};
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

    pub fn sub(&self, other: &Number) -> Number {
        match (self, other) {
            (Natural(l), Natural(r)) => Natural(l.saturating_sub(*r)),
            (l, r) => Decimal(l.as_f64() - r.as_f64()),
        }
    }

    pub fn mul(&self, other: &Number) -> Number {
        match (self, other) {
            (Natural(l), Natural(r)) => Natural(l.saturating_mul(*r)),
            (l, r) => Decimal(l.as_f64() * r.as_f64()),
        }
    }

    pub fn div(&self, other: &Number) -> Number {
        match (self, other) {
            (Natural(l), Natural(r)) => Natural(*l / *r),
            (l, r) => Decimal(l.as_f64() / r.as_f64()),
        }
    }

    pub fn modulo(&self, other: &Number) -> Number {
        match (self, other) {
            (Natural(l), Natural(r)) => Natural(l % *r),
            (l, r) => Decimal(l.as_f64() % r.as_f64()),
        }
    }

    pub fn pow(&self, other: &Number) -> Number {
        match (self, other) {
            (Natural(l), Natural(r)) => Natural(l.pow((*r).try_into().unwrap_or(u32::MAX))),
            (l, r) => Decimal(l.as_f64().pow(r.as_f64())),
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Natural(l), Natural(r)) => l.eq(r),
            (l, r) => l.as_f64().eq(&r.as_f64()),
        }
    }
}

impl Eq for Number {}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Natural(l), Natural(r)) => l.cmp(r),
            (l, r) => l.as_f64().partial_cmp(&r.as_f64()).unwrap(),
        }
    }
}

// what this asserts is that f64::partial_cmp is total when NaN is removed
#[test]
fn making_sure_i_am_not_dumb() {
    let mut v = vec![];

    // negative infinity
    let n = f64::NEG_INFINITY;
    assert!(f64::is_infinite(n));
    assert!(f64::is_sign_negative(n));
    v.push(n);

    // negative number
    let n = -1234.0;
    assert!(f64::is_finite(n));
    assert!(f64::is_sign_negative(n));
    assert!(f64::is_normal(n));
    v.push(n);

    // negative subnormal number
    let n = -f64::MIN_POSITIVE / 2.0;
    assert!(f64::is_finite(n));
    assert!(f64::is_sign_negative(n));
    assert!(!f64::is_normal(n));
    v.push(n);

    // negative zero (because of course)
    let n = -0.0;
    assert!(f64::is_finite(n));
    assert!(f64::is_sign_negative(n));
    assert!(!f64::is_normal(n));
    v.push(n);

    // positive zero
    let n = 0.0;
    assert!(f64::is_finite(n));
    assert!(f64::is_sign_positive(n));
    assert!(!f64::is_normal(n));
    v.push(n);

    // positive subnormal number
    let n = f64::MIN_POSITIVE / 2.0;
    assert!(f64::is_finite(n));
    assert!(f64::is_sign_positive(n));
    assert!(!f64::is_normal(n));
    v.push(n);

    // positive number
    let n = 1234.0;
    assert!(f64::is_finite(n));
    assert!(f64::is_sign_positive(n));
    assert!(f64::is_normal(n));
    v.push(n);

    // positive infinity
    let n = f64::INFINITY;
    assert!(f64::is_infinite(n));
    assert!(f64::is_sign_positive(n));
    v.push(n);

    for i in 0..v.len() {
        for j in 0..v.len() {
            if i == 3 && j == 4 || i == 4 && j == 3 {
                assert_eq!(v[i].partial_cmp(&v[j]), Some(Ordering::Equal));
            } else {
                assert_eq!(v[i].partial_cmp(&v[j]), i.partial_cmp(&j), "i:{} j:{}", i, j);
            }
        }
    }
}
