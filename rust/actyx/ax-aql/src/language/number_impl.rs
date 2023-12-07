use super::{render::render_number, Num};
use num_traits::Pow;
use std::{cmp::Ordering, convert::TryFrom};
use Num::*;

#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
pub enum NumError {
    #[display(fmt = "integer overflow")]
    IntOverflow,
    #[display(fmt = "integer underflow")]
    IntUnderflow,
    #[display(fmt = "floating-point overflow")]
    FloatOverflow,
    #[display(fmt = "floating-point underflow")]
    FloatUnderflow,
    #[display(fmt = "division by zero")]
    DivByZero,
    #[display(fmt = "not a number")]
    NaN,
}
use NumError::*;

impl Num {
    fn as_f64(&self) -> f64 {
        match self {
            Decimal(d) => *d,
            Natural(n) => *n as f64,
        }
    }

    pub fn add(&self, other: &Num) -> Result<Num, NumError> {
        match (self, other) {
            (Natural(l), Natural(r)) => Ok(Natural(l.checked_add(*r).ok_or(IntOverflow)?)),
            (l, r) => decimal(l.as_f64() + r.as_f64()),
        }
    }

    pub fn sub(&self, other: &Num) -> Result<Num, NumError> {
        match (self, other) {
            (Natural(l), Natural(r)) => Ok(Natural(l.checked_sub(*r).ok_or(IntOverflow)?)),
            (l, r) => decimal(l.as_f64() - r.as_f64()),
        }
    }

    pub fn mul(&self, other: &Num) -> Result<Num, NumError> {
        match (self, other) {
            (Natural(l), Natural(r)) => Ok(Natural(l.checked_mul(*r).ok_or(IntOverflow)?)),
            (l, r) => decimal(l.as_f64() * r.as_f64()),
        }
    }

    pub fn div(&self, other: &Num) -> Result<Num, NumError> {
        match (self, other) {
            (Natural(l), Natural(r)) => Ok(Natural(l.checked_div(*r).ok_or(DivByZero)?)),
            (l, r) => decimal(l.as_f64() / r.as_f64()),
        }
    }

    pub fn modulo(&self, other: &Num) -> Result<Num, NumError> {
        match (self, other) {
            (Natural(l), Natural(r)) => Ok(Natural(*l % *r)),
            (l, r) => decimal(l.as_f64() % r.as_f64()),
        }
    }

    pub fn pow(&self, other: &Num) -> Result<Num, NumError> {
        match (self, other) {
            (Natural(l), Natural(r)) => {
                let exponent = u32::try_from(*r).map_err(|_| IntOverflow)?;
                Ok(Natural(l.checked_pow(exponent).ok_or(IntOverflow)?))
            }
            (l, r) => decimal(l.as_f64().pow(r.as_f64())),
        }
    }
}

fn decimal(f: f64) -> Result<Num, NumError> {
    if f.is_finite() {
        Ok(Decimal(f))
    } else if f < 0.0 {
        Err(FloatUnderflow)
    } else if f > 0.0 {
        Err(FloatOverflow)
    } else {
        Err(NaN)
    }
}

impl PartialEq for Num {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Natural(l), Natural(r)) => l.eq(r),
            (l, r) => l.as_f64().eq(&r.as_f64()),
        }
    }
}

impl Eq for Num {}

impl PartialOrd for Num {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Num {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Natural(l), Natural(r)) => l.cmp(r),
            (l, r) => l.as_f64().partial_cmp(&r.as_f64()).unwrap(),
        }
    }
}

impl std::fmt::Display for Num {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        render_number(f, self)
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
