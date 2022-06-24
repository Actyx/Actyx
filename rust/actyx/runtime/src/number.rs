use num_bigint::BigInt;

/// Representation of numeric values for the purpose of computation.
///
/// The rules for binary operations are:
///
/// - first bring both operands to the more precise of the two representations
/// - `i128` and `f64` use Rust native semantics; `i128` overflow spills into Decimal
/// - +/-/% just act on the mantissa
/// - * uses m1*m2 * base^(e1+e2)
/// - / keeps the precision of the divident and truncates
/// - ^ extends precision of exponent to at least 0 and uses m1^m2 * base^(e1*m2)
///   (in case of negative e2 followed by base^(-e2)-th root)
///
/// This implies the need for operators that convert to a specific precision.
pub enum Arithmetic {
    Int(i128),
    IEEE754(f64),
    Decimal(BigInt, i128),
    Float(BigInt, i128),
}
