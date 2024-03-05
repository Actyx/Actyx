use std::{
    borrow::{Borrow, Cow},
    fmt::Display,
};

use anyhow::Context;
use ax_aql::{Label, NonEmptyVec, Type, TypeAtom};
use cbor_data::{value::Number, CborBuilder, CborValue, Encoder};

// NOTE: check tarpaulin for test coverage, its always hard to ensure every path was checked and
// in this case, it would be really useful: https://github.com/xd009642/tarpaulin

/// Check if a CBOR value is null.
fn validate_null(value: &CborValue) -> Result<(), anyhow::Error> {
    if value.is_null() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be null"))
    }
}

/// Check if a CBOR value is a boolean or a boolean refinement (i.e. `true` or `false`).
fn validate_bool(value: &CborValue, bool_refinement: &Option<bool>) -> Result<(), anyhow::Error> {
    if let Some(value) = value.as_bool() {
        if let Some(bool_refinement) = bool_refinement {
            if value == *bool_refinement {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "type mismatch, expected value to be {}, received {} instead",
                    bool_refinement,
                    value
                ))
            }
        } else {
            Ok(())
        }
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be a boolean"))
    }
}

/// Check if a CBOR value is an integer or an integer refinement (e.g. `10`).
fn validate_number(value: &CborValue, number_refinement: &Option<u64>) -> Result<(), anyhow::Error> {
    if let Some(Number::Int(value)) = value.as_number() {
        if let Ok(value) = u64::try_from(*value) {
            if let Some(number_refinement) = number_refinement {
                if value == *number_refinement {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "type mismatch, expected value to be {}, received {} instead",
                        number_refinement,
                        value
                    ))
                }
            } else {
                Ok(())
            }
        } else {
            Err(anyhow::anyhow!(
                "type mismatch, expected value to be a 64-bit integer but received a 128-bit integer instead"
            ))
        }
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be an integer"))
    }
}

/// Check if a CBOR value is a timestamp.
fn validate_timestamp(value: &CborValue) -> Result<(), anyhow::Error> {
    if value.as_timestamp().is_some() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be a timestamp"))
    }
}

/// Check if a CBOR value is a string or a string refinement (e.g. "Hello").
fn validate_string(value: &CborValue, string_refinement: &Option<String>) -> Result<(), anyhow::Error> {
    if let Some(value) = value.as_str() {
        if let Some(refinement) = string_refinement {
            if value == refinement {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "type mismatch, expected value to be {}, received {} instead",
                    refinement,
                    value
                ))
            }
        } else {
            Ok(())
        }
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be a string"))
    }
}

/// Check if a CBOR value is an array. Can also be used to check for tuples (following RFC 7049).
fn validate_array(value: &CborValue) -> Result<(), anyhow::Error> {
    // NOTE(duarte): this validate is incomplete because we're not supporting subtyping, hence, we're just checking for arrays
    if value.as_array().is_some() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be an array"))
    }
}

fn validate_tuple(value: &CborValue, length: usize) -> Result<(), anyhow::Error> {
    if let Some(array) = value.as_array() {
        if array.len() != length {
            Err(anyhow::anyhow!(
                "type mismatch, expected tuple to have length {} got {} instead",
                length,
                array.len()
            ))
        } else {
            Ok(())
        }
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be an array"))
    }
}

/// Check if a CBOR value is a dictionary.
fn validate_dict(value: &CborValue) -> Result<(), anyhow::Error> {
    if value.as_dict().is_some() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be a dictionary"))
    }
}

// The idea is that this function will only check for Atoms,
// the issue is that a Record can nest arbitrarily deep and may use non-atom types
// ideally, I would like to _not_ use recursion since we already had the stack issue in other places;
// so we either ignore records here and handle them somewhere else or we handle them here, some how
fn validate_atom(value: &CborValue, ty: &TypeAtom) -> Result<(), anyhow::Error> {
    match ty {
        TypeAtom::Null => validate_null(value),
        TypeAtom::Bool(refinement) => validate_bool(value, refinement),
        TypeAtom::Number(refinement) => validate_number(value, refinement),
        TypeAtom::Timestamp => validate_timestamp(value),
        TypeAtom::String(refinement) => validate_string(value, refinement),
        TypeAtom::Universal => Ok(()),
    }
}

fn validate_record(value: &CborValue, ty: &NonEmptyVec<(Label, Type)>) -> Result<(), anyhow::Error> {
    if let Some(value) = value.as_dict() {
        for (label, ty) in ty.iter() {
            match label {
                Label::String(string) => {
                    let cbor = CborBuilder::new().encode_str(string);
                    let cbor = Cow::Owned(cbor);
                    if let Some(value) = value.get(&cbor) {
                        if let e @ Err(_) = validate(&value.decode(), ty) {
                            return e;
                        }
                    } else {
                        return Err(anyhow::anyhow!("label {} does not exist in record", string));
                    }
                }
                Label::Number(number) => {
                    let number = i128::from(*number);
                    let number = Number::Int(number);
                    let cbor = CborBuilder::new().encode_number(&number);
                    let cbor = Cow::Owned(cbor);
                    if let Some(value) = value.get(&cbor) {
                        if let e @ Err(_) = validate(&value.decode(), ty) {
                            return e;
                        }
                    } else {
                        return Err(anyhow::anyhow!("label {:?} does not exist in record", number));
                    }
                }
            }
        }
        Ok(())
    } else {
        Err(anyhow::anyhow!("expected a record (dict)"))
    }
}

// Using logic rules and De Morgan laws to rewrite intersections and unions before we reach this method
// would (probably) allow us to write a better (i.e. simpler) and more performant validator
fn validate_union(value: &CborValue, ty: &(Type, Type)) -> Result<(), anyhow::Error> {
    // Since the union operation is commutative, we always evaluate the most specific type first
    // TODO: add tests
    match ty.borrow() {
        // Atom
        (Type::Atom(left), Type::Atom(right)) => {
            validate_atom(value, &left).or_else(|err| validate_atom(value, &right).context(err))
        }
        (Type::Atom(atom), Type::Union(union)) | (Type::Union(union), Type::Atom(atom)) => {
            validate_atom(value, &atom).or_else(|err| validate_union(value, union).context(err))
        }
        (Type::Atom(atom), Type::Intersection(intersection)) | (Type::Intersection(intersection), Type::Atom(atom)) => {
            validate_atom(value, &atom).or_else(|err| validate_intersection(value, intersection).context(err))
        }
        (Type::Atom(atom), Type::Array(_)) | (Type::Array(_), Type::Atom(atom)) => {
            // Arrays and tuples since according to RFC 7049 they're "interchangeable"
            validate_atom(value, &atom).or_else(|err| validate_array(value).context(err))
        }
        (Type::Atom(atom), Type::Tuple(tuple)) | (Type::Tuple(tuple), Type::Atom(atom)) => {
            validate_atom(value, atom).or_else(|err| validate_tuple(value, tuple.len()).context(err))
        }
        (Type::Atom(atom), Type::Dict(_)) | (Type::Dict(_), Type::Atom(atom)) => {
            validate_atom(value, &atom).or_else(|err| validate_dict(value).context(err))
        }
        (Type::Atom(atom), Type::Record(record)) | (Type::Record(record), Type::Atom(atom)) => {
            validate_atom(value, atom).or_else(|err| validate_record(value, record).context(err))
        }
        (Type::Array(_), Type::Array(_)) => validate_array(value), // TODO: support sub-typing
        (Type::Array(_), Type::Union(union)) | (Type::Union(union), Type::Array(_)) => {
            validate_array(value).or_else(|err| validate_union(value, union).context(err))
        }
        (Type::Array(_), Type::Intersection(intersection)) | (Type::Intersection(intersection), Type::Array(_)) => {
            validate_array(value).or_else(|err| validate_intersection(value, intersection).context(err))
        }
        (Type::Array(_), Type::Dict(_)) | (Type::Dict(_), Type::Array(_)) => {
            validate_array(value).or_else(|err| validate_dict(value).context(err))
        }
        (Type::Array(_), Type::Record(record)) | (Type::Record(record), Type::Array(_)) => {
            validate_array(value).or_else(|err| validate_record(value, record).context(err))
        }
        (Type::Array(_), Type::Tuple(tuple)) | (Type::Tuple(tuple), Type::Array(_)) => {
            validate_tuple(value, tuple.len()).or_else(|err| validate_array(value).context(err))
        }
        (Type::Dict(_), Type::Dict(_)) => validate_dict(value), // TODO: add sub-typing
        (Type::Dict(_), Type::Union(union)) | (Type::Union(union), Type::Dict(_)) => {
            validate_dict(value).or_else(|err| validate_union(value, union).context(err))
        }
        (Type::Dict(_), Type::Intersection(intersection)) | (Type::Intersection(intersection), Type::Dict(_)) => {
            validate_dict(value).or_else(|err| validate_intersection(value, intersection).context(err))
        }
        (Type::Dict(_), Type::Tuple(tuple)) | (Type::Tuple(tuple), Type::Dict(_)) => {
            validate_dict(value).or_else(|err| validate_tuple(value, tuple.len()).context(err))
        }
        (Type::Dict(_), Type::Record(record)) | (Type::Record(record), Type::Dict(_)) => {
            validate_dict(value).or_else(|err| validate_record(value, record).context(err))
        }
        (Type::Tuple(left), Type::Tuple(right)) => {
            // TODO: add further sub-typing
            validate_tuple(value, left.len()).or_else(|err| validate_tuple(value, right.len()).context(err))
        }
        (Type::Tuple(tuple), Type::Union(union)) | (Type::Union(union), Type::Tuple(tuple)) => {
            validate_tuple(value, tuple.len()).or_else(|err| validate_union(value, union).context(err))
        }
        (Type::Tuple(tuple), Type::Intersection(intersection))
        | (Type::Intersection(intersection), Type::Tuple(tuple)) => {
            validate_tuple(value, tuple.len()).or_else(|err| validate_intersection(value, intersection).context(err))
        }
        (Type::Tuple(tuple), Type::Record(record)) | (Type::Record(record), Type::Tuple(tuple)) => {
            validate_tuple(value, tuple.len()).or_else(|err| validate_record(value, record).context(err))
        }
        (Type::Record(left), Type::Record(right)) => {
            validate_record(value, left).or_else(|err| validate_record(value, right).context(err))
        }
        (Type::Record(record), Type::Union(union)) | (Type::Union(union), Type::Record(record)) => {
            // This is kind of a bet that the union will be shallower than the record
            validate_union(value, union).or_else(|err| validate_record(value, record).context(err))
        }
        (Type::Record(record), Type::Intersection(intersection))
        | (Type::Intersection(intersection), Type::Record(record)) => {
            // This is kind of a bet that the intersection will be shallower than the record
            validate_intersection(value, intersection).or_else(|err| validate_record(value, record).context(err))
        }
        (Type::Union(left), Type::Union(right)) => {
            validate_union(value, left).or_else(|err| validate_union(value, right).context(err))
        }
        (Type::Union(union), Type::Intersection(intersection))
        | (Type::Intersection(intersection), Type::Union(union)) => {
            validate_union(value, union).or_else(|err| validate_intersection(value, intersection).context(err))
        }
        (Type::Intersection(left), Type::Intersection(right)) => {
            validate_intersection(value, left).or_else(|err| validate_intersection(value, right).context(err))
        }
    }
}

fn validate_intersection(value: &CborValue, ty: &(Type, Type)) -> Result<(), anyhow::Error> {
    match ty.borrow() {
        (Type::Atom(left), Type::Atom(right)) => {
            intersect_atoms(left, right).and_then(|intersection| validate_atom(value, &intersection))
        }
        (Type::Atom(atom), Type::Union(union)) | (Type::Union(union), Type::Atom(atom)) => {
            validate_atom(value, atom).and_then(|_| validate_union(value, union))
        }
        (Type::Atom(atom), Type::Intersection(intersection)) | (Type::Intersection(intersection), Type::Atom(atom)) => {
            validate_atom(value, atom).and_then(|_| validate_intersection(value, intersection))
        }
        (Type::Array(_), Type::Array(_)) => validate_array(value), // TODO: handle subtyping
        (Type::Array(_), Type::Union(union)) | (Type::Union(union), Type::Array(_)) => {
            validate_array(value).and_then(|_| validate_union(value, union))
        }
        (Type::Array(_), Type::Intersection(intersection)) | (Type::Intersection(intersection), Type::Array(_)) => {
            validate_array(value).and_then(|_| validate_intersection(value, intersection))
        }
        (Type::Dict(_), Type::Dict(_)) => validate_dict(value), // TODO: handle subtyping
        (Type::Dict(_), Type::Union(union)) | (Type::Union(union), Type::Dict(_)) => {
            validate_dict(value).and_then(|_| validate_union(value, union))
        }
        (Type::Dict(_), Type::Intersection(intersection)) | (Type::Intersection(intersection), Type::Dict(_)) => {
            validate_dict(value).and_then(|_| validate_intersection(value, intersection))
        }
        (Type::Tuple(left), Type::Tuple(right)) => {
            // TODO: add subtyping
            if left.len() == right.len() {
                validate_tuple(value, left.len())
            } else {
                Err(anyhow::anyhow!("invalid intersection"))
            }
        }
        (Type::Tuple(tuple), Type::Union(union)) | (Type::Union(union), Type::Tuple(tuple)) => {
            validate_tuple(value, tuple.len()).and_then(|_| validate_union(value, union))
        }
        (Type::Tuple(tuple), Type::Intersection(intersection))
        | (Type::Intersection(intersection), Type::Tuple(tuple)) => {
            validate_tuple(value, tuple.len()).and_then(|_| validate_intersection(value, intersection))
        }
        (Type::Record(left), Type::Record(right)) => {
            // TODO: we can intersect the records to get either a single one or an error
            // such approach should make this more efficient
            validate_record(value, left).and_then(|_| validate_record(value, right))
        }
        (Type::Record(record), Type::Union(union)) | (Type::Union(union), Type::Record(record)) => {
            validate_record(value, record).and_then(|_| validate_union(value, union))
        }
        (Type::Record(record), Type::Intersection(intersection))
        | (Type::Intersection(intersection), Type::Record(record)) => {
            validate_record(value, record).and_then(|_| validate_intersection(value, intersection))
        }
        (Type::Union(left), Type::Union(right)) => {
            validate_union(value, left).and_then(|_| validate_union(value, right))
        }
        (Type::Intersection(left), Type::Intersection(right)) => {
            validate_intersection(value, left).and_then(|_| validate_intersection(value, right))
        }
        (Type::Union(union), Type::Intersection(intersection))
        | (Type::Intersection(intersection), Type::Union(union)) => {
            // Assuming that the intersection will result in an acceptance/rejection "faster"
            validate_intersection(value, intersection).and_then(|_| validate_union(value, union))
        }
        _ => {
            // All other intersections are invalid!
            // Theoretically, this should be unreachable as it should be caught earlier by the type checker
            Err(anyhow::anyhow!("invalid intersection"))
        }
    }
}

fn validate(value: &CborValue, ty: &Type) -> Result<(), anyhow::Error> {
    match ty {
        Type::Atom(atom) => validate_atom(value, atom),
        Type::Union(union) => validate_union(value, union),
        Type::Intersection(intersection) => validate_intersection(value, intersection),
        Type::Array(_) => validate_array(value),
        Type::Dict(_) => validate_dict(value),
        Type::Tuple(tuple) => validate_tuple(value, tuple.len()),
        Type::Record(record) => validate_record(value, record),
    }
}

/// Intersect refinements. Intersecting two absent refinements or an absent refinement and an existing one is always successful.
///
/// For example:
/// ```text
/// BOOL & BOOL => BOOL
/// true & BOOL => true
/// true & true => true
/// true & false => !
/// ```
fn intersect_refinement<T: Eq + Display + Clone>(
    left: &Option<T>,
    right: &Option<T>,
) -> Result<Option<T>, anyhow::Error> {
    match (left, right) {
        (None, None) => Ok(None),
        (None, Some(r)) => Ok(Some(r.clone())),
        (Some(l), None) => Ok(Some(l.clone())),
        (Some(l), Some(r)) => {
            if l == r {
                Ok(Some(l.clone()))
            } else {
                Err(anyhow::anyhow!("failed to intersect {} and {}", l, r))
            }
        }
    }
}

// NOTE(duarte): ideally, some of these methods, instead of returning an error, should return a "Never" type
// that then gets translated to an error, in the most adequate place

/// Intersect atoms. As a rule of thumb, if your atoms are not of the same kind they will not intersect, furthermore,
/// if both atoms are refinable see [`intersect_refinement`] for more information.
fn intersect_atoms(left: &TypeAtom, right: &TypeAtom) -> Result<TypeAtom, anyhow::Error> {
    match (left, right) {
        (TypeAtom::Bool(l), TypeAtom::Bool(r)) => intersect_refinement(l, r).map(|r| (TypeAtom::Bool(r))),
        (TypeAtom::String(l), TypeAtom::String(r)) => intersect_refinement(l, r).map(|r| (TypeAtom::String(r))),
        (TypeAtom::Number(l), TypeAtom::Number(r)) => intersect_refinement(l, r).map(|r| (TypeAtom::Number(r))),
        // This one also covers Universal & Universal
        (TypeAtom::Universal, atom) | (atom, TypeAtom::Universal) => Ok(atom.clone()),
        // TODO(duarte): figure out a better error message
        (left, right) => Err(anyhow::anyhow!("failed to intersect {:?} and {:?}", left, right)),
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::runtime::operation::validation::validate_union;

    use super::{
        validate_array, validate_atom, validate_bool, validate_dict, validate_intersection, validate_null,
        validate_number, validate_record, validate_string, validate_timestamp, validate_tuple,
    };

    use ax_aql::{Label, Type, TypeAtom};
    use cbor_data::{
        value::{Precision, Timestamp},
        CborBuilder, Encoder,
    };

    #[test]
    fn test_validate_null() {
        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();
        assert!(validate_null(&cbor).is_ok());
    }

    #[test]
    fn test_validate_null_fail() {
        let cbor = CborBuilder::new().encode_i64(100);
        let cbor = cbor.decode();
        assert!(validate_null(&cbor).is_err());
    }

    #[test]
    fn test_validate_bool() {
        let cbor = CborBuilder::new().encode_bool(false);
        let cbor = cbor.decode();
        assert!(validate_bool(&cbor, &None).is_ok());
        assert!(validate_bool(&cbor, &Some(false)).is_ok());
        assert!(validate_bool(&cbor, &Some(true)).is_err());
    }

    #[test]
    fn test_validate_bool_fail() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();
        assert!(validate_bool(&cbor, &None).is_err());
        assert!(validate_bool(&cbor, &Some(false)).is_err());
    }

    #[test]
    fn test_validate_number() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();
        assert!(validate_number(&cbor, &None).is_ok());
        assert!(validate_number(&cbor, &Some(10)).is_ok());
        assert!(validate_number(&cbor, &Some(50)).is_err());
    }

    #[test]
    fn test_validate_number_fail() {
        let cbor = CborBuilder::new().encode_f64(0.100);
        let cbor = cbor.decode();
        assert!(validate_number(&cbor, &None).is_err());
        assert!(validate_number(&cbor, &Some(100)).is_err());
    }

    #[test]
    fn test_validate_timestamp() {
        let timestamp = Timestamp::new(876523558, 0, 0);
        let cbor = CborBuilder::new().encode_timestamp(timestamp, Precision::Seconds);
        let cbor = cbor.decode();
        assert!(validate_timestamp(&cbor).is_ok());
    }

    #[test]
    fn test_validate_timestamp_fail() {
        let cbor = CborBuilder::new().encode_str("value");
        let cbor = cbor.decode();
        assert!(validate_timestamp(&cbor).is_err());
    }

    #[test]
    fn test_validate_string() {
        let cbor = CborBuilder::new().encode_str("Olá mundo!");
        let cbor = cbor.decode();
        assert!(validate_string(&cbor, &None).is_ok());
        assert!(validate_string(&cbor, &Some("Olá mundo!".to_string())).is_ok());
        assert!(validate_string(&cbor, &Some("Adeus mundo!".to_string())).is_err());
    }

    #[test]
    fn test_validate_string_fail() {
        let cbor = CborBuilder::new().encode_i64(64);
        let cbor = cbor.decode();
        assert!(validate_string(&cbor, &None).is_err());
        assert!(validate_string(&cbor, &Some("Adeus mundo!".to_string())).is_err());
    }

    #[test]
    fn test_validate_universal() {
        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();
        assert!(validate_atom(&cbor, &TypeAtom::Universal).is_ok());

        let cbor = CborBuilder::new().encode_bool(true);
        let cbor = cbor.decode();
        assert!(validate_atom(&cbor, &TypeAtom::Universal).is_ok());

        let cbor = CborBuilder::new().encode_i64(100);
        let cbor = cbor.decode();
        assert!(validate_atom(&cbor, &TypeAtom::Universal).is_ok());

        let timestamp = Timestamp::new(876523558, 0, 0);
        let cbor = CborBuilder::new().encode_timestamp(timestamp, Precision::Seconds);
        let cbor = cbor.decode();
        assert!(validate_atom(&cbor, &TypeAtom::Universal).is_ok());

        let cbor = CborBuilder::new().encode_str("Hello!");
        let cbor = cbor.decode();
        assert!(validate_atom(&cbor, &TypeAtom::Universal).is_ok());
    }

    #[test]
    fn test_validate_array() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_u64(10).encode_u64(100);
        });
        let cbor = cbor.decode();
        assert!(validate_array(&cbor).is_ok());
    }

    #[test]
    fn test_validate_array_fail() {
        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_key("hello", |b| b.encode_str("world"));
        });
        let cbor = cbor.decode();
        assert!(validate_array(&cbor).is_err());
    }

    #[test]
    fn test_validate_tuple() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_u64(10).encode_u64(100);
        });
        let cbor = cbor.decode();
        assert!(validate_tuple(&cbor, 2).is_ok());
    }

    #[test]
    fn test_validate_tuple_fail() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_u64(10).encode_u64(100);
        });
        let cbor = cbor.decode();
        assert!(validate_tuple(&cbor, 4).is_err());

        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_key("hello", |b| b.encode_str("world"));
        });
        let cbor = cbor.decode();
        assert!(validate_tuple(&cbor, 3).is_err());
    }

    #[test]
    fn test_validate_dict() {
        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_key("hello", |b| b.encode_str("world"));
        });
        let cbor = cbor.decode();
        assert!(validate_dict(&cbor).is_ok());
    }

    #[test]
    fn test_validate_dict_fail() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_u64(10).encode_u64(100);
        });
        let cbor = cbor.decode();
        assert!(validate_dict(&cbor).is_err());
    }

    #[test]
    fn test_validate_record() {
        // { "temperature": 100, { "coordinates": { "x": 10, "y": -10 } } }
        let cbor = CborBuilder::new().encode_dict(|builder| {
            // TODO: number support
            builder.with_key("temperature", |builder| builder.encode_i64(100));
            builder.with_key("coordinates", |builder| {
                builder.encode_dict(|builder| {
                    builder.with_key("x", |builder| builder.encode_i64(10));
                    builder.with_key("y", |builder| builder.encode_i64(10));
                })
            });
        });
        let cbor = cbor.decode();
        let ty = vec![
            (
                Label::String("temperature".to_string().try_into().expect("non-empty string")),
                Type::Atom(TypeAtom::Number(None)),
            ),
            (
                Label::String("coordinates".to_string().try_into().expect("non-empty string")),
                Type::Record(
                    vec![
                        (
                            Label::String("x".to_string().try_into().expect("non-empty string")),
                            Type::Atom(TypeAtom::Number(None)),
                        ),
                        (
                            Label::String("y".to_string().try_into().expect("non-empty string")),
                            Type::Atom(TypeAtom::Number(None)),
                        ),
                    ]
                    .try_into()
                    .expect("proper type"),
                ),
            ),
        ]
        .try_into()
        .expect("proper type");
        validate_record(&cbor, &ty).unwrap();
        // assert!(validate_record(&cbor, &ty).is_ok());
    }

    #[test]
    fn test_validate_union_simple() {
        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();
        let left = Type::Atom(TypeAtom::Null);
        let right = Type::Atom(TypeAtom::Timestamp);
        assert!(validate_union(&cbor, &(left.clone(), right.clone())).is_ok());
        assert!(validate_union(&cbor, &(right.clone(), left.clone())).is_ok());
    }

    #[test]
    fn test_validate_union_nested_bi() {
        let left = {
            let left = Type::Atom(TypeAtom::Null);
            let right = Type::Atom(TypeAtom::Timestamp);
            Arc::new((left, right))
        };

        let right = {
            let left = Type::Atom(TypeAtom::Bool(Some(false)));
            let right = Type::Atom(TypeAtom::String(Some("Olá!".to_string())));
            Arc::new((left, right))
        };

        let cbor = CborBuilder::new().encode_str("Olá!");
        let cbor = cbor.decode();

        // Null | Timestamp
        assert!(validate_union(&cbor, left.as_ref()).is_err());
        // Bool(false) | String("Olá")
        assert!(validate_union(&cbor, right.as_ref()).is_ok());
        // (Null | Timestamp) | (Bool(false) | String("Olá"))
        assert!(validate_union(&cbor, &((Type::Union(left), Type::Union(right)))).is_ok());
    }

    #[test]
    fn test_validate_union_nested_leaning() {
        // A | (B | (C | D)

        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();

        // Null | Timestamp
        let first_union = {
            let left = Type::Atom(TypeAtom::Null);
            let right = Type::Atom(TypeAtom::Timestamp);
            Arc::new((left, right))
        };
        assert!(validate_union(&cbor, first_union.as_ref()).is_ok());

        // Boolean(false) | (Null | Timestamp)
        let second_union = {
            let left = Type::Atom(TypeAtom::Bool(Some(false)));
            let right = Type::Union(first_union);
            Arc::new((left, right))
        };
        assert!(validate_union(&cbor, second_union.as_ref()).is_ok());

        // String("Olá") | (Boolean(false) | (Null | Timestamp))
        let third_union = {
            let left = Type::Atom(TypeAtom::String(Some("Olá".to_string())));
            let right = Type::Union(second_union);
            Arc::new((left, right))
        };
        assert!(validate_union(&cbor, third_union.as_ref()).is_ok());
    }

    #[test]
    fn test_validate_union_nested_leaning_fail() {
        // A | (B | (C | D)

        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();

        // Null | Timestamp
        let first_union = {
            let left = Type::Atom(TypeAtom::Number(Some(10)));
            let right = Type::Atom(TypeAtom::Timestamp);
            Arc::new((left, right))
        };
        assert!(validate_union(&cbor, first_union.as_ref()).is_err());

        // Boolean(false) | (Null | Timestamp)
        let second_union = {
            let left = Type::Atom(TypeAtom::Bool(Some(false)));
            let right = Type::Union(first_union);
            Arc::new((left, right))
        };
        assert!(validate_union(&cbor, second_union.as_ref()).is_err());

        // String("Olá") | (Boolean(false) | (Null | Timestamp))
        let third_union = {
            let left = Type::Atom(TypeAtom::String(Some("Olá".to_string())));
            let right = Type::Union(second_union);
            Arc::new((left, right))
        };
        assert!(validate_union(&cbor, third_union.as_ref()).is_err());
    }

    #[test]
    fn test_validate_intersection() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();
        let left = Type::Atom(TypeAtom::Number(None));
        let right = Type::Atom(TypeAtom::Number(Some(10)));

        assert!(validate_intersection(&cbor, &(left, right)).is_ok());
    }

    #[test]
    fn test_validate_intersection_fail() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();
        let left = Type::Atom(TypeAtom::Null);
        let right = Type::Atom(TypeAtom::Timestamp);
        assert!(validate_intersection(&cbor, &(left, right)).is_err_and(|err| {
            let err_message = format!("failed to intersect {:?} and {:?}", TypeAtom::Null, TypeAtom::Timestamp);
            err.to_string() == err_message
        }));
    }
}
